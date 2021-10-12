use portaudio::PortAudio;
use portaudio as pa;
use std::error::Error;
use log::info;
use portaudio::stream::OutputSettings;

// PortAudio constants
const INTERLEAVED: bool = true;
const LATENCY: pa::Time = 0.0; // Ignored by PortAudio::is_*_format_supported.
const FRAMES_PER_BUFFER: u32 = 64;
const SAMPLE_RATE: f64 = 48000.0;


pub fn list_audio_devices(pa: &PortAudio) -> Result<i32, Box<dyn Error>> {
    let num_devices = pa.device_count()?;
    info!("Number of audio devices = {}", num_devices);

    for device in pa.devices()? {
        let (idx, info) = device?;

        let in_channels = info.max_input_channels;
        let input_params = pa::StreamParameters::<i16>::new(idx, in_channels, INTERLEAVED, LATENCY);
        let out_channels = info.max_output_channels;
        let output_params =
            pa::StreamParameters::<i16>::new(idx, out_channels, INTERLEAVED, LATENCY);
        let in_48k_supported = pa.is_input_format_supported(input_params, SAMPLE_RATE).is_ok();
        let out_48k_supported = pa.is_output_format_supported(output_params, SAMPLE_RATE).is_ok();
        let support_48k = if (in_channels > 0 && in_48k_supported) || (out_channels > 0 && out_48k_supported) { "48000Hz supported" } else { "48000Hz not supported" };
        info!("{:?}: {:?} / IN:{} OUT:{} @ {}Hz default; {}", idx.0, info.name, info.max_input_channels,
            info.max_output_channels, info.default_sample_rate, support_48k);
    }
    Ok(0)
}

pub fn output_audio_device_exists(pa: &PortAudio, dev_name: &str) -> Result<bool, Box<dyn Error>> {
    for device in pa.devices()? {
        let (idx, info) = device?;

        let out_channels = info.max_output_channels;
        let output_params =
            pa::StreamParameters::<i16>::new(idx, out_channels, INTERLEAVED, LATENCY);
        let out_48k_supported = pa.is_output_format_supported(output_params, SAMPLE_RATE).is_ok();
        if info.name == dev_name && out_channels > 0 && out_48k_supported {
            return Ok(true)
        }
    }
    Ok(false)
}

pub fn input_audio_device_exists(pa: &PortAudio, dev_name: &str) -> Result<bool, Box<dyn Error>> {
    for device in pa.devices()? {
        let (idx, info) = device?;

        let in_channels = info.max_input_channels;
        let input_params =
            pa::StreamParameters::<i16>::new(idx, in_channels, INTERLEAVED, LATENCY);
        let in_48k_supported = pa.is_input_format_supported(input_params, SAMPLE_RATE).is_ok();
        if info.name == dev_name && in_channels > 0 && in_48k_supported {
            return Ok(true)
        }
    }
    Ok(false)
}

pub fn open_output_audio_device(pa: &PortAudio, dev_name: &str) -> Result<OutputSettings<i16>, Box<dyn Error>> {
    for device in pa.devices()? {
        let (idx, info) = device?;

        let out_channels = info.max_output_channels;
        let output_params =
            pa::StreamParameters::<i16>::new(idx, out_channels, INTERLEAVED, LATENCY);
        let out_48k_supported = pa.is_output_format_supported(output_params, SAMPLE_RATE).is_ok();
        if info.name == dev_name && out_channels > 0 && out_48k_supported {
            let settings = OutputSettings::new(output_params, SAMPLE_RATE, FRAMES_PER_BUFFER);
            return Ok(settings);
        }
    }
    Err(Box::<dyn Error + Send + Sync>::from(format!("Can't find settings for device '{}'", dev_name)))
}