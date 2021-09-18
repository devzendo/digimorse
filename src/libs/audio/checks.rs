
/* Bill Somerville on the WSJT-X mailing list says, on sample rates:
   "WSJT-X requests a 48 kHz 16-bit audio stream for input and it generates output in the same
   format. The reason we suggest you use 48 kHz as the default sample rate is because operating
   system re-sampling is prone to audio artefacts that can degrade the receive audio performance.
   We actually re-sample in WSJT-X down to 12 kHz before the DSP processing which gives us a
   bandwidth of up to 6 kHz, the down sampling in WSJT-X uses a high quality algorithm but it is
   always better to do integral factor re-sampling so an input sample rate that is an exact power
   of two of the requested rate is most efficient."
 */
use portaudio::PortAudio;
use portaudio as pa;
use std::error::Error;
use log::info;

// PortAudio constants
const INTERLEAVED: bool = true;
const LATENCY: pa::Time = 0.0; // Ignored by PortAudio::is_*_format_supported.


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
        let in_48k_supported = pa.is_input_format_supported(input_params, 48000.0).is_ok();
        let out_48k_supported = pa.is_output_format_supported(output_params, 48000.0).is_ok();
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
        let out_48k_supported = pa.is_output_format_supported(output_params, 48000.0).is_ok();
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
        let in_48k_supported = pa.is_input_format_supported(input_params, 48000.0).is_ok();
        if info.name == dev_name && in_channels > 0 && in_48k_supported {
            return Ok(true)
        }
    }
    Ok(false)
}
