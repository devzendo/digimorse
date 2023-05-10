#[macro_use]
extern crate clap;
extern crate portaudio;

use std::error::Error;
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;
use std::thread;
use std::time::Duration;

use clap::{App, Arg, ArgMatches};
use clap::arg_enum;
use digimorse::libs::keyer_io::null_keyer_io::NullKeyer;
use fltk::app;
use log::{debug, error, info, warn};
use fltk::app::*;
use portaudio::PortAudio;
use syncbox::ScheduledThreadPool;

use digimorse::libs::config_dir::config_dir;
use digimorse::libs::keyer_io::arduino_keyer_io::ArduinoKeyer;
use digimorse::libs::keyer_io::keyer_io::{MAX_KEYER_SPEED, MIN_KEYER_SPEED, Keyer};
use digimorse::libs::keyer_io::keyer_io::{KeyerSpeed, KeyerType};
use digimorse::libs::serial_io::serial_io::{DefaultSerialIO, NullSerialIO, SerialIO};
use digimorse::libs::util::util::printable;
use digimorse::libs::application::application::{Application, ApplicationMode};
use digimorse::libs::config_file::config_file::ConfigurationStore;
use digimorse::libs::audio::audio_devices::{list_audio_devices, output_audio_device_exists, input_audio_device_exists, list_audio_input_devices, list_audio_output_devices};
use digimorse::libs::audio::tone_generator::ToneGenerator;
use digimorse::libs::channel_codec::channel_encoder::{ChannelEncoder, source_encoding_to_channel_encoding};
use digimorse::libs::channel_codec::ldpc::init_ldpc;
use digimorse::libs::gui::gui::Gui;
use digimorse::libs::gui::gui_facades::GUIOutput;
use digimorse::libs::source_codec::source_encoder::SourceEncoder;
use digimorse::libs::source_codec::source_encoding::{SOURCE_ENCODER_BLOCK_SIZE_IN_BITS};
use digimorse::libs::transmitter::transmitter::{AmplitudeMax, Transmitter};
use digimorse::libs::util::logging::initialise_logging;
use digimorse::libs::util::version::VERSION;

#[cfg(windows)]
const KEYER_HELP: &str = "Sets the port that the Digimorse Arduino Keyer is connected to, e.g. COM4:";
#[cfg(windows)]
const KEYER_VALUE_NAME: &str = "COM port";

#[cfg(not(windows))]
const KEYER_HELP: &str = "Sets the port that the Digimorse Arduino Keyer is connected to, e.g. /dev/cu-usbserial-1410";
#[cfg(not(windows))]
const KEYER_VALUE_NAME: &str = "serial character device";

const NO_KEYER: &'static str = "no-keyer";
const KEYER_PORT_DEVICE: &'static str = "keyer-port-device";
const AUDIO_OUT_DEVICE: &'static str = "audio-out-device";
const RIG_OUT_DEVICE: &'static str = "rig-out-device";
const RIG_IN_DEVICE: &'static str = "rig-in-device";
const KEYER_SPEED_WPM: &'static str = "keyer-speed-wpm";

arg_enum! {
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum Mode {
        GUI,
        ConfigFileLocation,
        ListAudioDevices,
        ListOutputDevices,
        ListInputDevices,
        SerialDiag,
        SourceEncoderDiag // TODO remove when moved to diag_application_spec
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

        .arg(Arg::with_name(NO_KEYER)
             .short("n")
             .long("nokeyer")
             .help("Clears the keyer device; the GUI would be used for text entry"))

        .arg(Arg::with_name(AUDIO_OUT_DEVICE)
            .short("a").long("audioout").help("Sets the audio device name to use for the speaker/headphone output")
            .value_name("speaker/headphone audio output device name").takes_value(true))

        .arg(Arg::with_name(RIG_OUT_DEVICE)
            .short("t").long("rigaudioout").help("Sets the audio device name to use for output to the transceiver")
            .value_name("transceiver audio output device name").takes_value(true))

        .arg(Arg::with_name(RIG_IN_DEVICE)
            .short("r").long("rigaudioin").help("Sets the audio device name to use for input from the transceiver")
            .value_name("transceiver audio input device name").takes_value(true))

        .arg(Arg::with_name(KEYER_SPEED_WPM)
            .short("w").long("keyerwpm").help("Sets the typical keying speed in words per minute")
            .value_name("keyer speed in WPM").takes_value(true))

        .get_matches();

    let mode = value_t!(result.value_of("mode"), Mode).unwrap_or(Mode::GUI);

    return (result, mode);
}

fn run(arguments: ArgMatches, mode: Mode) -> Result<i32, Box<dyn Error>> {
    let home_dir = dirs::home_dir();
    let config_path = config_dir::configuration_directory(home_dir)?;
    let config_path_clone = config_path.clone();
    let mut config = ConfigurationStore::new(config_path).unwrap();
    let config_file_path = config.get_config_file_path();
    if mode == Mode::ConfigFileLocation {
        info!("Configuration path is [{:?}]", config_path_clone);
        info!("Configuration file is [{:?}]", config_file_path);
        return Ok(0)
    }
    let pa = PortAudio::new()?;

    match mode {
        Mode::ListAudioDevices => {
            list_audio_devices(&pa)?;
            return Ok(0)
        }
        Mode::ListInputDevices => {
            list_audio_input_devices(&pa)?;
            return Ok(0)
        }
        Mode::ListOutputDevices => {
            list_audio_output_devices(&pa)?;
            return Ok(0)
        }
        _ => {}
    }

    // Eventually device and settings configuration will be via a nice GUI. Until then, have options
    // on the command line that will set the various devices in the configuration file, and then
    // pick the values from config to initialise the system, after checking that these configured
    // values are still valid.
    configure_audio_and_keyer_devices(&arguments, &mut config, &pa)?;

    // Examine configured audio and keyer devices (may be repeating checks just made if they're
    // being set, or checking what was previously configured).
    check_audio_devices(&mut config, &pa)?;
    check_keyer_device(&mut config)?;

    fn construct_default_serial_io(port: &str) -> Result<Box<dyn SerialIO>, String> {
        match DefaultSerialIO::new(port.to_string()) {
            Ok(serial_io) => { Ok(Box::new(serial_io)) }
            Err(e) => { Err(e) }
        }
    }

    fn construct_null_serial_io() -> Result<Box<dyn SerialIO>, String> {
        match NullSerialIO::new() {
            Ok(serial_io) => { Ok(Box::new(serial_io)) }
            Err(e) => { Err(e) }
        }
    }

    let mut box_serial_io: Box<dyn SerialIO> = if config.get_keyer_type() == KeyerType::Arduino {
        let port_string = config.get_port();
        let port = port_string.as_str();
        info!("Initialising serial port at {}", port);
        construct_default_serial_io(port)?
    } else {
        construct_null_serial_io()?
    };

    if mode == Mode::SerialDiag {
        serial_diag(&mut box_serial_io)?;
        return Ok(0)
    }

    // From here, we need wiring, so, an Application that handles the 'wiring loom' for the given
    // mode makes sense. The major components (depending on the mode) are instantiated, and set in
    // the Application, which wires them up correctly.
    // TODO the various diags are being moved to manually-invoked tests and they will not be
    // invokable from the command line. This main will have config-setup commands, and the main
    // proper.

    let terminate = Arc::new(AtomicBool::new(false));
    let scheduled_thread_pool = Arc::new(ScheduledThreadPool::single_thread());
    info!("Initialising Application...");
    let mut application = Application::new(terminate.clone(), scheduled_thread_pool.clone(), pa);
    application.set_ctrlc_handler();
    match mode {
        Mode::GUI => {
            application.set_mode(ApplicationMode::Full);
        }
        Mode::SourceEncoderDiag => {
            application.set_mode(ApplicationMode::SourceEncoderDiag); // TODO remove this when source encoder diag moved to diag_application_spec
        }
        _ => {}
    }

    info!("Initialising keyer...");

    fn construct_arduino_keyer(box_serial_io: Box<dyn SerialIO>, terminate_flag: Arc<AtomicBool>) -> Arc<Mutex<dyn Keyer>> {
        Arc::new(Mutex::new(ArduinoKeyer::new(box_serial_io, terminate_flag)))
    }
    fn construct_null_keyer() -> Arc<Mutex<dyn Keyer>> {
        Arc::new(Mutex::new(NullKeyer::new()))
    }

    let keyer = if config.get_keyer_type() == KeyerType::Arduino {
        construct_arduino_keyer(box_serial_io, application.terminate_flag())
    } else {
        construct_null_keyer()
    };
    let keyer_speed: KeyerSpeed = config.get_wpm() as KeyerSpeed;
    application.set_keyer_speed(keyer_speed);
    application.set_keyer(keyer); // This also sets the speed on the keyer.

    info!("Initialising audio output (from the computer, ie its speaker)...");
    let out_dev_string = config.get_audio_out_device();
    let out_dev_str = out_dev_string.as_str();
    let output_settings = application.open_output_audio_device(out_dev_str)?;
    let mut tone_generator = ToneGenerator::new(config.get_sidetone_frequency(),
                                                application.terminate_flag());
    tone_generator.start_callback(application.pa_ref(), output_settings)?; // also initialises DDS for sidetone.
    let application_tone_generator = Arc::new(Mutex::new(tone_generator));
    application.set_tone_generator(application_tone_generator);

    info!("Initialising source encoder...");
    let mut source_encoder = SourceEncoder::new(application.terminate_flag(),
                                                SOURCE_ENCODER_BLOCK_SIZE_IN_BITS);
    // TODO the application should set the source encoder's speed.
    source_encoder.set_keyer_speed(config.get_wpm() as KeyerSpeed);
    application.set_source_encoder(Arc::new(Mutex::new(source_encoder)));

    // These devices have been previously checked for existence..
    info!("Initialising rig input (from the rig, ie its speaker) device...");
    let rig_in_dev_string = config.get_rig_in_device();
    let rig_in_dev_str = rig_in_dev_string.as_str();
    let _rig_input_settings = application.open_input_audio_device(rig_in_dev_str).unwrap();

    info!("Initialising rig output (to the rig, ie its mic) device...");
    let rig_out_dev_string = config.get_rig_out_device();
    let rig_out_dev_str = rig_out_dev_string.as_str();
    let rig_output_settings = application.open_output_audio_device(rig_out_dev_str).unwrap();

    info!("Initialising LDPC...");
    init_ldpc();

    info!("Initialising channel encoder...");
    let channel_encoder = Arc::new(Mutex::new(ChannelEncoder::new(source_encoding_to_channel_encoding, application.terminate_flag())));
    application.set_channel_encoder(channel_encoder);

    info!("Initialising transmitter...");
    let transmitter = Arc::new(Mutex::new(Transmitter::new(config.get_transmit_offset_frequency(), application.terminate_flag())));
    application.set_transmitter(transmitter.clone());

    {
        let mut locked_transmitter = transmitter.lock().unwrap();
        info!("Setting amplitude max");
        locked_transmitter.set_amplitude_max(config.get_transmit_amplitude() as AmplitudeMax);
        info!("Initialising transmitter audio callback...");
        locked_transmitter.start_callback(application.pa_ref(), rig_output_settings)?;
        info!("Setting transmitter offset audio frequency...");
        locked_transmitter.set_audio_frequency_allocate_buffer(config.get_transmit_offset_frequency());
    }

    info!("Initialising GUI...");
    let gui_config = Arc::new(Mutex::new(config));
    let gui_terminate = application.terminate_flag();
    let gui_application = Arc::new(Mutex::new(application));
    let terminate_application = gui_application.clone();
    let app = fltk::app::App::default().with_scheme(Scheme::Gtk);
    let gui = Gui::new(gui_config, gui_application, gui_terminate);
    let gui_input = gui.gui_input_sender();
    let arc_mutex_gui = Arc::new(Mutex::new(gui));
    // The gui_input would be passed around to other parts of the subsystem.
    {
        let mut locked_transmitter = transmitter.lock().unwrap();
        let transmitter_gui_input = gui_input.clone();
        locked_transmitter.set_gui_input(transmitter_gui_input);
    }

    while app.wait() {
        arc_mutex_gui.lock().unwrap().message_handle();
    }
    info!("End of GUI message loop; terminating");
    terminate_application.lock().unwrap().terminate();
    thread::sleep(Duration::from_secs(2));
    info!("Exiting");
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

    let mut keyer_device_ok = true;
    // Set the keyer port in the configuration file, if present.
    if arguments.is_present(KEYER_PORT_DEVICE) && arguments.is_present(NO_KEYER) {
        warn!("Cannot use {} and {} options", KEYER_PORT_DEVICE, NO_KEYER);
        keyer_device_ok = false;
    } else {
        if arguments.is_present(KEYER_PORT_DEVICE) {
            let dev = arguments.value_of(KEYER_PORT_DEVICE).unwrap();
            let exists = port_exists(dev)?;
            if exists {
                info!("Setting keyer serial port device to '{}'", dev);
                config.set_port(dev.to_string())?;
                config.set_keyer_type(KeyerType::Arduino)?;
            } else {
                warn!("Setting {}: No keyer serial port device named '{}' is present in your system.", KEYER_PORT_DEVICE, dev);
                keyer_device_ok = false;
            }
        }
        if arguments.is_present(NO_KEYER) {
            info!("Clearing any keyer serial port device");
            config.set_port("".to_string())?;
            config.set_keyer_type(KeyerType::Null)?;
        }
    }

    // Set the keyer speed in the configuration file, if present.
    if arguments.is_present(KEYER_SPEED_WPM) {
        let wpm_str = arguments.value_of(KEYER_SPEED_WPM).unwrap();
        match wpm_str.parse::<usize>() {
            Ok(wpm) => {
                if wpm >= MIN_KEYER_SPEED as usize && wpm <= MAX_KEYER_SPEED as usize {
                    info!("Setting keyer speed to {} WPM", wpm);
                    config.set_wpm(wpm)?;
                } else {
                    warn!("Setting {}: Keyer speed of {} is out of the range [{}..{}] WPM", KEYER_SPEED_WPM, wpm, MIN_KEYER_SPEED, MAX_KEYER_SPEED);
                    keyer_device_ok = false;
                }
            }
            Err(_) => {
                warn!("Setting {}: Could not set keyer speed in WPM to '{}' - not an integer", KEYER_SPEED_WPM, wpm_str);
                keyer_device_ok = false;
            }
        }
    }

    if !keyer_device_ok {
        return Err("Configuration error in keyer device.".into());
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
    if config.get_keyer_type() == KeyerType::Arduino {
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
    }

    if keyer_ok {
        Ok(())
    } else {
        Err("Configuration error checking keyer device.".into())
    }
}

#[cfg(windows)]
fn port_exists(dev_name: &str) -> Result<bool, Box<dyn Error>> {
    let ports = serialport::available_ports()?;
    for p in ports {
        if p.port_name == dev_name.to_string() {
            return Ok(true);
        }
    }
    return Ok(false);
}

#[cfg(not(windows))]
fn port_exists(dev_name: &str) -> Result<bool, Box<dyn Error>> {
    Ok(std::path::Path::new(dev_name).exists())
}

fn serial_diag(serial_io: &mut Box<dyn SerialIO>) -> Result<(), Box<dyn Error>> {
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

fn main() {
    initialise_logging();

    let (arguments, mode) = parse_command_line();
    debug!("Command line parsed");

    if mode == Mode::GUI {
        let _app = app::App::default().with_scheme(app::Scheme::Gleam);
        // TODO this should be passed to the GUI code.
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

