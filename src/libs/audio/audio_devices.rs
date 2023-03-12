use portaudio::{InputStreamSettings, OutputStreamSettings, PortAudio};
use portaudio as pa;
use std::error::Error;
use log::info;
use regex::Regex;
use simple_error::bail;

// PortAudio constants
const INTERLEAVED: bool = true;
const LATENCY: pa::Time = 0.0; // Ignored by PortAudio::is_*_format_supported.
pub(crate) const FRAMES_PER_BUFFER: u32 = 64; // May have to increase this to 1024
pub(crate) const SAMPLE_RATE: f64 = 48000.0;


pub fn list_audio_devices(pa: &PortAudio) -> Result<i32, Box<dyn Error>> {
    let num_devices = pa.device_count()?;
    info!("Number of audio devices = {}", num_devices);

    for device in pa.devices()? {
        let (idx, info) = device?;

        let in_channels = info.max_input_channels;
        let input_params = pa::StreamParameters::<i16>::new(idx, in_channels, INTERLEAVED, LATENCY);
        let out_channels = info.max_output_channels;
        let output_params =
            pa::StreamParameters::<f32>::new(idx, out_channels, INTERLEAVED, LATENCY);
        let in_48k_supported = pa.is_input_format_supported(input_params, SAMPLE_RATE).is_ok();
        let out_48k_supported = pa.is_output_format_supported(output_params, SAMPLE_RATE).is_ok();
        let support_48k = if (in_channels > 0 && in_48k_supported) || (out_channels > 0 && out_48k_supported) { "48000Hz supported" } else { "48000Hz not supported" };
        info!("{:?}: {:?} / IN:{} OUT:{} @ {}Hz default; {}", idx.0, info.name, info.max_input_channels,
            info.max_output_channels, info.default_sample_rate, support_48k);
    }
    Ok(0)
}

pub fn list_audio_input_devices(pa: &PortAudio) -> Result<i32, Box<dyn Error>> {
    for device in pa.devices()? {
        let (idx, info) = device?;

        let in_channels = info.max_input_channels;
        if in_channels > 0 {
            let input_params = pa::StreamParameters::<i16>::new(idx, in_channels, INTERLEAVED, LATENCY);
            let in_48k_supported = pa.is_input_format_supported(input_params, SAMPLE_RATE).is_ok();
            if in_48k_supported {
                info!("{:?}: {:?} / IN:{} @ {}Hz default", idx.0, info.name, info.max_input_channels, info.default_sample_rate);
            }
        }
    }
    Ok(0)
}

pub fn list_audio_output_devices(pa: &PortAudio) -> Result<i32, Box<dyn Error>> {
    for device in pa.devices()? {
        let (idx, info) = device?;

        let out_channels = info.max_output_channels;
        if out_channels > 0 {
            let output_params =
                pa::StreamParameters::<f32>::new(idx, out_channels, INTERLEAVED, LATENCY);
            let out_48k_supported = pa.is_output_format_supported(output_params, SAMPLE_RATE).is_ok();
            if out_48k_supported {
                info!("{:?}: {:?} / OUT:{} @ {}Hz default", idx.0, info.name, info.max_output_channels, info.default_sample_rate);
            }
        }
    }
    Ok(0)
}

pub(crate) fn parse_dev_name(dev_name: &str) -> Result<(Option<u32>, String), Box<dyn Error>> {
    let re = Regex::new(r"^(?:(\d*)\s*:)?\s*([^:].*)$")?;
    match re.captures(dev_name) {
        None => {
            bail!("Device name does not match pattern [number:] name");
        }
        Some(caps) => {
            let maybe_index_str = caps.get(1);
            let maybe_device_str = caps.get(2);

            if maybe_index_str.is_some() && maybe_index_str.unwrap().as_str() == "" {
                bail!("Missing device index number at start of '{}'", dev_name);
            }
            let maybe_index = maybe_index_str.map(|d| d.as_str().to_string().parse::<u32>().unwrap());
            // unwrap since if present the regex guarantees it's digits - (ignore out of range for usize)
            let device_name = maybe_device_str.map_or("", |m| m.as_str()).to_string();
            Ok((maybe_index, device_name))
        }
    }
}

// The dev_name may be prefixed with num: in which case this must match the device index.
pub fn output_audio_device_exists(pa: &PortAudio, dev_name: &str) -> Result<bool, Box<dyn Error>> {
    let (maybe_idx, name) = parse_dev_name(dev_name)?;

    for device in pa.devices()? {
        let (idx, info) = device?;

        let out_channels = info.max_output_channels;
        let output_params =
            pa::StreamParameters::<f32>::new(idx, out_channels, INTERLEAVED, LATENCY);
        let out_48k_supported = pa.is_output_format_supported(output_params, SAMPLE_RATE).is_ok();
        let idx_matches = maybe_idx.is_none() || (maybe_idx.unwrap() == idx.0);
        if idx_matches && info.name == name && out_channels > 0 && out_48k_supported {
            return Ok(true)
        }
    }
    Ok(false)
}

// The dev_name may be prefixed with num: in which case this must match the device index.
pub fn input_audio_device_exists(pa: &PortAudio, dev_name: &str) -> Result<bool, Box<dyn Error>> {
    let (maybe_idx, name) = parse_dev_name(dev_name)?;

    for device in pa.devices()? {
        let (idx, info) = device?;

        let in_channels = info.max_input_channels;
        let input_params =
            pa::StreamParameters::<f32>::new(idx, in_channels, INTERLEAVED, LATENCY);
        let in_48k_supported = pa.is_input_format_supported(input_params, SAMPLE_RATE).is_ok();
        let idx_matches = maybe_idx.is_none() || (maybe_idx.unwrap() == idx.0);
        if idx_matches && info.name == name && in_channels > 0 && in_48k_supported {
            return Ok(true)
        }
    }
    Ok(false)
}

pub fn open_output_audio_device(pa: &PortAudio, dev_name: &str) -> Result<OutputStreamSettings<f32>, Box<dyn Error>> {
    let (maybe_idx, name) = parse_dev_name(dev_name)?;

    for device in pa.devices()? {
        let (idx, info) = device?;

        let out_channels = info.max_output_channels;
        let output_params =
            pa::StreamParameters::<f32>::new(idx, out_channels, INTERLEAVED, LATENCY);
        let out_48k_supported = pa.is_output_format_supported(output_params, SAMPLE_RATE).is_ok();
        let idx_matches = maybe_idx.is_none() || (maybe_idx.unwrap() == idx.0);
        if idx_matches && name == info.name && out_channels > 0 && out_48k_supported {
            info!("Using {:?} as audio output device", info);
            let settings = OutputStreamSettings::new(output_params, SAMPLE_RATE, FRAMES_PER_BUFFER);
            return Ok(settings);
        }
    }
    Err(Box::<dyn Error + Send + Sync>::from(format!("Can't find output settings for device '{}'", dev_name)))
}

pub fn open_input_audio_device(pa: &PortAudio, dev_name: &str) -> Result<InputStreamSettings<f32>, Box<dyn Error>> {
    let (maybe_idx, name) = parse_dev_name(dev_name)?;

    for device in pa.devices()? {
        let (idx, info) = device?;

        let in_channels = info.max_input_channels;
        let input_params =
            pa::StreamParameters::<f32>::new(idx, in_channels, INTERLEAVED, LATENCY);
        let in_48k_supported = pa.is_input_format_supported(input_params, SAMPLE_RATE).is_ok();
        let idx_matches = maybe_idx.is_none() || (maybe_idx.unwrap() == idx.0);
        if idx_matches && name == info.name && in_channels > 0 && in_48k_supported {
            let settings = InputStreamSettings::new(input_params, SAMPLE_RATE, FRAMES_PER_BUFFER);
            return Ok(settings);
        }
    }
    Err(Box::<dyn Error + Send + Sync>::from(format!("Can't find input settings for device '{}'", dev_name)))
}

#[cfg(test)]
#[path = "./audio_devices_spec.rs"]
mod audio_devices_spec;
