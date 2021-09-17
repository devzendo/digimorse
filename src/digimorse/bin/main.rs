#[macro_use]
extern crate clap;
extern crate portaudio;

use clap::{App, Arg, ArgMatches};
use fltk::{app, prelude::*, window::Window};
use log::{debug, error, info, warn};
use portaudio as pa;

use std::path::{PathBuf, Path};
use std::fs;
use std::env;
use std::any::Any;
use std::error::Error;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::{mpsc, Mutex};

use digimorse::libs::config_dir::config_dir;
use digimorse::libs::keyer_io::arduino_keyer_io::ArduinoKeyer;
use digimorse::libs::keyer_io::keyer_io::KeyingEvent;
use digimorse::libs::keyer_io::keyer_io::KeyerSpeed;
use digimorse::libs::serial_io::serial_io::{DefaultSerialIO, SerialIO};
use digimorse::libs::source_encoder::source_encoder::DefaultSourceEncoder;
use digimorse::libs::util::util::printable;

use std::time::Duration;
use portaudio::PortAudio;
use digimorse::libs::config_file::config_file::ConfigurationStore;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[cfg(windows)]
const KEYER_HELP: &str = "Sets the port that the Digimorse Arduino Keyer is connected to, e.g. COM4:";
#[cfg(windows)]
const KEYER_VALUE_NAME: &str = "COM port";

#[cfg(not(windows))]
const KEYER_HELP: &str = "Sets the port that the Digimorse Arduino Keyer is connected to, e.g. /dev/cu-usbserial-1410";
#[cfg(not(windows))]
const KEYER_VALUE_NAME: &str = "Serial character device";

// TODO Not sure what a suitable port name for Linux would be

const AUDIO_OUT_DEVICE: &'static str = "audio-out-device";
const RIG_OUT_DEVICE: &'static str = "rig-out-device";
const RIG_IN_DEVICE: &'static str = "rig-in-device";

// PortAudio constants
const INTERLEAVED: bool = true;
const LATENCY: pa::Time = 0.0; // Ignored by PortAudio::is_*_format_supported.

arg_enum! {
    #[derive(Debug, Clone, PartialEq)]
    enum Mode {
        GUI,
        ListAudioDevices,
        SerialDiag,
        KeyerDiag,
        SourceEncoderDiag
    }
}

fn parse_command_line<'a>() -> (ArgMatches<'a>, Mode) {
    let result = App::new("digimorse")
        .version(VERSION)
        .author("Matt Gumbley <matt.gumbley@gmail.com>")
        .about("Digitally Encoded Morse Transceiver")

        .arg(Arg::from_usage("<mode> 'The mode to use, usually GUI.'").possible_values(&Mode::variants()).default_value("GUI"))

        .arg(Arg::with_name("keyer-port")
            .short("k")
            .long("keyer")
            .value_name(KEYER_VALUE_NAME)
            .help(KEYER_HELP)
            .takes_value(true))

        .arg(Arg::with_name(AUDIO_OUT_DEVICE)
            .short("a").long("audioout").help("Sets the audio device name to use for the speaker/headphone output")
            .value_name("speaker/headphone audio output device name").takes_value(true))

        .arg(Arg::with_name(RIG_OUT_DEVICE)
            .short("t").long("rigaudioout").help("Sets the audio device name to use for output to the transceiver")
            .value_name("transceiver audio output device name").takes_value(true))

        .arg(Arg::with_name(RIG_IN_DEVICE)
            .short("r").long("rigaudioin").help("Sets the audio device name to use for input from the transceiver")
            .value_name("transceiver audio input device name").takes_value(true))

        .get_matches();

    let mode = value_t!(result.value_of("mode"), Mode).unwrap_or(Mode::GUI);

    return (result, mode);
}

fn initialise_logging() {
    let log_var_name = "RUST_LOG";
    if env::var(log_var_name).is_err() {
        env::set_var(log_var_name, "info")
    }
    env_logger::init();
}

fn run(arguments: ArgMatches, mode: Mode) -> Result<i32, Box<dyn Error>> {
    let home_dir = dirs::home_dir();
    let config_path = config_dir::configuration_directory(home_dir)?;
    debug!("Configuration path is [{:?}]", config_path);
    let mut config = ConfigurationStore::new(config_path).unwrap();
    debug!("Configuration file is [{:?}]", config.get_config_file_path());

    let pa = pa::PortAudio::new()?;
    if mode == Mode::ListAudioDevices {
        list_audio_devices(&pa)?;
        return Ok(0)
    }

    // Set any audio devices in the configuration, if present.
    let mut set_audio_ok = true;
    if arguments.is_present(AUDIO_OUT_DEVICE) {
        let dev = arguments.value_of(AUDIO_OUT_DEVICE).unwrap();
        let exists = output_audio_device_exists(&pa, dev)?;
        if exists {
            config.set_audio_out_device(dev.to_string());
        } else {
            warn!("Setting {}: No output audio device named '{}' is present in your system.", AUDIO_OUT_DEVICE, dev);
            set_audio_ok = false;
        }
    }
    if arguments.is_present(RIG_OUT_DEVICE) {
        let dev = arguments.value_of(RIG_OUT_DEVICE).unwrap();
        let exists = output_audio_device_exists(&pa, dev)?;
        if exists {
            config.set_rig_out_device(dev.to_string());
        } else {
            warn!("Setting {}: No output audio device named '{}' is present in your system.", RIG_OUT_DEVICE, dev);
            set_audio_ok = false;
        }
    }
    if arguments.is_present(RIG_IN_DEVICE) {
        let dev = arguments.value_of(RIG_IN_DEVICE).unwrap();
        let exists = input_audio_device_exists(&pa, dev)?;
        if exists {
            config.set_rig_in_device(dev.to_string());
        } else {
            warn!("Setting {}: No input audio device named '{}' is present in your system.", RIG_IN_DEVICE, dev);
            set_audio_ok = false;
        }
    }

    // Examine configured audio devices (may be repeating checks just made if they're being set, or
    // checking what was previously configured).
    {
        let dev = config.get_audio_out_device().as_str();
        let exists = output_audio_device_exists(&pa, dev)?;
        if !exists {
            warn!("Checking {}: No output audio device named '{}' is present in your system.", AUDIO_OUT_DEVICE, dev);
            set_audio_ok = false;
        }
    }
    {
        let dev = config.get_rig_out_device().as_str();
        let exists = output_audio_device_exists(&pa, dev)?;
        if !exists {
            warn!("Checking {}: No output audio device named '{}' is present in your system.", RIG_OUT_DEVICE, dev);
            set_audio_ok = false;
        }
    }
    {
        let dev = config.get_rig_in_device().as_str();
        let exists = input_audio_device_exists(&pa, dev)?;
        if !exists {
            warn!("Checking {}: No input audio device named '{}' is present in your system.", RIG_IN_DEVICE, dev);
            set_audio_ok = false;
        }
    }

    if !set_audio_ok {
        return Err("Configuration error in audio devices".into())
    }


    // TODO get port from the configuration file
    let port = "/dev/tty.usbserial-1420".to_string();
    info!("Initialising serial port at {}", port);
    let mut serial_io = DefaultSerialIO::new(port)?;
    if mode == Mode::SerialDiag {
        serial_diag(&mut serial_io)?;
        return Ok(0)
    }

    info!("Initialising keyer...");
    let (keying_event_tx, keying_event_rx): (Sender<KeyingEvent>, Receiver<KeyingEvent>) = mpsc::channel();
    let mut keyer = ArduinoKeyer::new(Box::new(serial_io), keying_event_tx);
    if mode == Mode::KeyerDiag {
        loop {
            let result = keying_event_rx.recv_timeout(Duration::from_millis(250));
            match result {
                Ok(keying_event) => {
                    info!("Keying Event {}", keying_event);
                }
                Err(err) => {
                    // be quiet, it's ok..
                }
            }
        }
    }

    info!("Initialising source encoder...");
    // TODO get WPM from the configuration file
    // TODO ARCHITECTURE need a backbone/application to which various subsystems/implementations or
    // implementations with modified configuration are attached dynamically at runtime (and can be
    // changed by the preferences dialog, etc.)
    let keyer_speed: KeyerSpeed = 20;
    let mut source_encoder = DefaultSourceEncoder::new(keying_event_rx);
    if mode == Mode::SourceEncoderDiag {

    }

    Ok(0)
}

fn serial_diag(serial_io: &mut DefaultSerialIO) -> Result<i32, Box<dyn Error>> {
    loop {
        let mut read_buf: [u8; 1] = [0];
        let read_bytes = serial_io.read(&mut read_buf);
        match read_bytes {
            Ok(1) => {
                info!("read {}", printable(read_buf[0]));
            }
            Ok(n) => {
                warn!("In build loop, received {} bytes, but should be only 1?!", n);
            }
            Err(_) => {
                // Be silent when there's nothing incoming..
            }
        }
    }

}

/* Bill Somerville on the WSJT-X mailing list says, on sample rates:
   "WSJT-X requests a 48 kHz 16-bit audio stream for input and it generates output in the same
   format. The reason we suggest you use 48 kHz as the default sample rate is because operating
   system re-sampling is prone to audio artefacts that can degrade the receive audio performance.
   We actually re-sample in WSJT-X down to 12 kHz before the DSP processing which gives us a
   bandwidth of up to 6 kHz, the down sampling in WSJT-X uses a high quality algorithm but it is
   always better to do integral factor re-sampling so an input sample rate that is an exact power
   of two of the requested rate is most efficient."
 */
fn list_audio_devices(pa: &PortAudio) -> Result<i32, Box<dyn Error>> {
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

fn output_audio_device_exists(pa: &PortAudio, dev_name: &str) -> Result<bool, Box<dyn Error>> {
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

fn input_audio_device_exists(pa: &PortAudio, dev_name: &str) -> Result<bool, Box<dyn Error>> {
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

fn main() {
    initialise_logging();

    let (arguments, mode) = parse_command_line();
    debug!("Command line parsed");

    if mode == Mode::GUI {
        let app = app::App::default().with_scheme(app::Scheme::Gleam);
    }

    match run(arguments, mode.clone()) {
        Err(err) => {
            match mode {
                Mode::GUI => {
                    fltk::dialog::message_default(&*format!("{}", err));
                }
                _ => {
                    error!("{}", err);
                }
            }
        }
        Ok(exit_code) => {
            std::process::exit(exit_code);
        }
    }
}

