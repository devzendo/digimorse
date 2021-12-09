#[macro_use]
extern crate clap;
extern crate portaudio;

use core::mem;
use clap::{App, Arg, ArgMatches};
use fltk::app;
use log::{debug, error, info, warn};
use portaudio as pa;
use pretty_hex::*;

use std::{env, thread};
use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use digimorse::libs::config_dir::config_dir;
use digimorse::libs::keyer_io::arduino_keyer_io::ArduinoKeyer;
use digimorse::libs::keyer_io::keyer_io::{Keyer, KeyingEvent};
use digimorse::libs::keyer_io::keyer_io::KeyerSpeed;
use digimorse::libs::serial_io::serial_io::{DefaultSerialIO, SerialIO};
use digimorse::libs::util::util::printable;

use std::time::Duration;
use bus::{Bus, BusReader};
use csv::Writer;
use portaudio::PortAudio;
use digimorse::libs::config_file::config_file::ConfigurationStore;
use digimorse::libs::audio::audio_devices::{list_audio_devices, output_audio_device_exists, input_audio_device_exists, open_output_audio_device};
use digimorse::libs::audio::tone_generator::ToneGenerator;
use digimorse::libs::source_codec::source_encoder::SourceEncoder;
use digimorse::libs::source_codec::source_encoding::SourceEncoding;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[cfg(windows)]
const KEYER_HELP: &str = "Sets the port that the Digimorse Arduino Keyer is connected to, e.g. COM4:";
#[cfg(windows)]
const KEYER_VALUE_NAME: &str = "COM port";

#[cfg(not(windows))]
const KEYER_HELP: &str = "Sets the port that the Digimorse Arduino Keyer is connected to, e.g. /dev/cu-usbserial-1410";
#[cfg(not(windows))]
const KEYER_VALUE_NAME: &str = "serial character device";

const KEYER_PORT_DEVICE: &'static str = "keyer-port-device";
const AUDIO_OUT_DEVICE: &'static str = "audio-out-device";
const RIG_OUT_DEVICE: &'static str = "rig-out-device";
const RIG_IN_DEVICE: &'static str = "rig-in-device";

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

        .arg(Arg::with_name(KEYER_PORT_DEVICE)
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

    // Eventually device configuration will be via a nice GUI. Until then, have options on the
    // command line that will set the various devices in the configuration file, and then pick the
    // values from config to initialise the system, after checking that these configured values are
    // still valid.
    configure_audio_and_keyer_devices(&arguments, &mut config, &pa)?;

    // Examine configured audio and keyer devices (may be repeating checks just made if they're
    // being set, or checking what was previously configured).
    check_audio_devices(&mut config, &pa)?;
    check_keyer_device(&mut config)?;

    let port_string = config.get_port();
    let port = port_string.as_str();

    info!("Initialising serial port at {}", port);
    let mut serial_io = DefaultSerialIO::new(port.to_string())?;

    if mode == Mode::SerialDiag {
        serial_diag(&mut serial_io)?;
        return Ok(0)
    }

    info!("Initialising keyer...");
    let mut keying_event_tx = Bus::new(16);
    let tone_generator_keying_event_rx = keying_event_tx.add_rx();
    let keyer_diag_keying_event_rx: Option<BusReader<KeyingEvent>> = if mode == Mode::KeyerDiag {
        Some(keying_event_tx.add_rx())
    } else {
        None
    };
    let source_encoder_keying_event_rx: Option<BusReader<KeyingEvent>> = if mode == Mode::KeyerDiag {
        None
    } else {
        Some(keying_event_tx.add_rx())
    };

    let terminate = Arc::new(AtomicBool::new(false));
    let mut keyer = ArduinoKeyer::new(Box::new(serial_io), keying_event_tx, terminate.clone());
    let keyer_speed: KeyerSpeed = config.get_wpm() as KeyerSpeed;
    keyer.set_speed(keyer_speed)?;

    let ctrlc_arc_terminate = terminate.clone();
    ctrlc::set_handler(move || {
        info!("Setting terminate flag...");
        ctrlc_arc_terminate.store(true, Ordering::SeqCst);
        info!("... terminate flag set");
    }).expect("Error setting Ctrl-C handler");

    info!("Initialising audio callback...");
    let dev_string = config.get_audio_out_device();
    let dev = dev_string.as_str();
    let output_settings = open_output_audio_device(&pa, dev)?;
    let mut tone_generator = ToneGenerator::new(config.get_sidetone_frequency(),
                                                tone_generator_keying_event_rx, terminate.clone());
    tone_generator.start_callback(&pa, output_settings)?;

    if mode == Mode::KeyerDiag {
        info!("Initialising KeyerDiag mode");
        keyer_diag(keyer_diag_keying_event_rx.unwrap(), terminate.clone())?;
        keyer.terminate();
        mem::drop(tone_generator);
        pa.terminate()?;
        thread::sleep(Duration::from_secs(1));
        info!("Finishing KeyerDiag mode");
        return Ok(0);
    }

    info!("Initialising source encoder...");
    // TODO ARCHITECTURE need a backbone/application to which various subsystems/implementations or
    // implementations with modified configuration are attached dynamically at runtime (and can be
    // changed by the preferences dialog, etc.)


    let mut source_encoder_tx = Bus::new(16);
    let source_encoder_rx = source_encoder_tx.add_rx();
    let source_encoder = SourceEncoder::new(source_encoder_keying_event_rx.unwrap(), source_encoder_tx, terminate.clone());

    if mode == Mode::SourceEncoderDiag {
        info!("Initialising SourceEncoderDiag mode");
        source_encoder_diag(source_encoder_rx, terminate.clone())?;
        keyer.terminate();
        mem::drop(tone_generator);
        pa.terminate()?;
        thread::sleep(Duration::from_secs(1));
        info!("Finishing SourceEncoderDiag mode");
        return Ok(0);
    }

    Ok(0)
}

fn configure_audio_and_keyer_devices(arguments: &ArgMatches, config: &mut ConfigurationStore, pa: &PortAudio) -> Result<(), Box<dyn Error>> {
    let mut audio_devices_ok = true;

    // Set any audio devices in the configuration, if present.
    if arguments.is_present(AUDIO_OUT_DEVICE) {
        let dev = arguments.value_of(AUDIO_OUT_DEVICE).unwrap();
        let exists = output_audio_device_exists(&pa, dev)?;
        if exists {
            info!("Setting audio output device to '{}'", dev);
            config.set_audio_out_device(dev.to_string())?;
        } else {
            warn!("Setting {}: No output audio device named '{}' is present in your system.", AUDIO_OUT_DEVICE, dev);
            audio_devices_ok = false;
        }
    }
    if arguments.is_present(RIG_OUT_DEVICE) {
        let dev = arguments.value_of(RIG_OUT_DEVICE).unwrap();
        let exists = output_audio_device_exists(&pa, dev)?;
        if exists {
            info!("Setting rig output device to '{}'", dev);
            config.set_rig_out_device(dev.to_string())?;
        } else {
            warn!("Setting {}: No output audio device named '{}' is present in your system.", RIG_OUT_DEVICE, dev);
            audio_devices_ok = false;
        }
    }
    if arguments.is_present(RIG_IN_DEVICE) {
        let dev = arguments.value_of(RIG_IN_DEVICE).unwrap();
        let exists = input_audio_device_exists(&pa, dev)?;
        if exists {
            info!("Setting audio input device to '{}'", dev);
            config.set_rig_in_device(dev.to_string())?;
        } else {
            warn!("Setting {}: No input audio device named '{}' is present in your system.", RIG_IN_DEVICE, dev);
            audio_devices_ok = false;
        }
    }

    if !audio_devices_ok {
        return Err("Configuration error when setting audio devices. To show current audio devices, use the ListAudioDevices mode.".into())
    }

    // Set the port in the configuration file, if present.
    if arguments.is_present(KEYER_PORT_DEVICE) {
        let dev = arguments.value_of(KEYER_PORT_DEVICE).unwrap();
        let exists = port_exists(dev)?;
        if exists {
            info!("Setting keyer serial port device to '{}'", dev);
            config.set_port(dev.to_string())?;
        } else {
            warn!("Setting {}: No keyer serial port device named '{}' is present in your system.", KEYER_PORT_DEVICE, dev);
            return Err("Configuration error in keyer device.".into());
        }
    }

    Ok(())
}

fn check_audio_devices(config: &mut ConfigurationStore, pa: &PortAudio) -> Result<(), Box<dyn Error>> {
    let mut audio_devices_ok = true;
    {
        let dev_string = config.get_audio_out_device();
        let dev = dev_string.as_str();
        if dev.is_empty() {
            warn!("No audio output device has been configured; use the -a or --audioout options");
            audio_devices_ok = false;
        } else {
            let exists = output_audio_device_exists(&pa, dev)?;
            if !exists {
                warn!("Checking {}: No output audio device named '{}' is present in your system.", AUDIO_OUT_DEVICE, dev);
                audio_devices_ok = false;
            }
            info!("Audio output device is '{}'", dev);
        }
    }
    {
        let dev_string = config.get_rig_out_device();
        let dev = dev_string.as_str();
        if dev.is_empty() {
            warn!("No rig output device has been configured; use the -t or --rigaudioout options");
            audio_devices_ok = false;
        } else {
            let exists = output_audio_device_exists(&pa, dev)?;
            if !exists {
                warn!("Checking {}: No output audio device named '{}' is present in your system.", RIG_OUT_DEVICE, dev);
                audio_devices_ok = false;
            }
            info!("Rig output device is '{}'", dev);
        }
    }
    {
        let dev_string = config.get_rig_in_device();
        let dev = dev_string.as_str();
        if dev.is_empty() {
            warn!("No rig input device has been configured; use the -r or --rigaudioin options");
            audio_devices_ok = false;
        } else {
            let exists = input_audio_device_exists(&pa, dev)?;
            if !exists {
                warn!("Checking {}: No input audio device named '{}' is present in your system.", RIG_IN_DEVICE, dev);
                audio_devices_ok = false;
            }
            info!("Rig input device is '{}'", dev);
        }
    }

    if audio_devices_ok {
        Ok(())
    } else {
        Err("Configuration error when checking audio devices. To show current audio devices, use the ListAudioDevices mode.".into())
    }
}

fn check_keyer_device(config: &mut ConfigurationStore) -> Result<(), Box<dyn Error>> {
    let mut keyer_ok = true;
    let port_string = config.get_port();
    let port = port_string.as_str();
    if port.is_empty() {
        warn!("No keyer serial port device has been configured; use the -k or --keyer options");
        keyer_ok = false;
    } else {
        let port_exists = port_exists(port)?;
        if !port_exists {
            warn!("Checking {}: No keyer serial port device named '{}' is present in your system.", KEYER_PORT_DEVICE, port);
            keyer_ok = false;
        }
        info!("Keyer serial port device is '{}'", port);
    }

    if keyer_ok {
        Ok(())
    } else {
        Err("Configuration error checking keyer device.".into())
    }
}

fn port_exists(dev_name: &str) -> Result<bool, Box<dyn Error>> {
    // Might have to do something funky on Windows to check whether COMx: exists? Would this suffice?
    Ok(std::path::Path::new(dev_name).exists())
}

fn serial_diag(serial_io: &mut DefaultSerialIO) -> Result<(), Box<dyn Error>> {
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

fn keyer_diag(mut keying_event_rx: BusReader<KeyingEvent>, terminate: Arc<AtomicBool>) -> Result<(), Box<dyn Error>> {
    let mut wtr = Writer::from_path("keying.csv")?;
    loop {
        if terminate.load(Ordering::SeqCst) {
            break;
        }
        let result = keying_event_rx.recv_timeout(Duration::from_millis(250));
        match result {
            Ok(keying_event) => {
                info!("KeyerDiag: Keying Event {}", keying_event);
                match keying_event {
                    // KeyingTimedEvents give the duration at the END of a (mark|space). If the
                    // key is now up, then we've just heard a mark (key down), and if it's now down,
                    // we've just heard a space (key up).
                    // If we see a start, that's just the starting key down edge of a mark; an
                    // end is actually meaningless in terms of keying - it's just a timeout after
                    // the user has ended keying. In terms of generating a histogram of
                    // keying, the stream should be a single long over - ie no END/STARTs in the
                    // middle - otherwise you'll see two consecutive MARKs, which makes no sense.
                    KeyingEvent::Timed(timed) => {
                        wtr.write_record(&[if timed.up { "MARK" } else { "SPACE" }, format!("{}", timed.duration).as_str()])?;
                        wtr.flush()?;
                    }
                    KeyingEvent::Start() => {}
                    KeyingEvent::End() => {}
                }
            }
            Err(_) => {
                // be quiet, it's ok..
            }
        }
    }
    info!("KeyerDiag: terminating");
    return Ok(());
}

fn source_encoder_diag(mut source_encoder_rx: BusReader<SourceEncoding>, terminate: Arc<AtomicBool>) -> Result<(), Box<dyn Error>> {
    loop {
        if terminate.load(Ordering::SeqCst) {
            break;
        }
        let result = source_encoder_rx.recv_timeout(Duration::from_millis(250));
        match result {
            Ok(source_encoding) => {
                info!("SourceEncodingDiag: isEnd {}", source_encoding.is_end);
                let hexdump = pretty_hex(&source_encoding.block);
                let hexdump_lines = hexdump.split("\n");
                for line in hexdump_lines {
                    info!("SourceEncodingDiag: Encoding {}", line);
                }
            }
            Err(_) => {
                // be quiet, it's ok..
            }
        }
    }
    info!("SourceEncodingDiag: terminating");
    return Ok(());
}

fn main() {
    initialise_logging();

    let (arguments, mode) = parse_command_line();
    debug!("Command line parsed");

    if mode == Mode::GUI {
        let _app = app::App::default().with_scheme(app::Scheme::Gleam);
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

