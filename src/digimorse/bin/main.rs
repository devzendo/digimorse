#[macro_use]
extern crate clap;

use clap::{App, Arg, ArgMatches};
use fltk::{app, prelude::*, window::Window};
use log::{debug, error, info, warn};

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

arg_enum! {
    #[derive(Debug, Clone, PartialEq)]
    enum Mode {
        GUI,
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

        .arg(Arg::with_name("keyer port")
            .short("k")
            .long("keyer")
            .value_name(KEYER_VALUE_NAME)
            .help(KEYER_HELP)
            .takes_value(true))

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
    info!("Configuration path is [{:?}]", config_path);

    // TODO get port from the configuration file
    let port = "/dev/tty.usbserial-1420".to_string();
    info!("Initialising serial port at {}", port);
    let mut serial_io = DefaultSerialIO::new(port)?;
    if (mode == Mode::SerialDiag) {
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

    info!("Initialising keyer...");
    let (keying_event_tx, keying_event_rx): (Sender<KeyingEvent>, Receiver<KeyingEvent>) = mpsc::channel();
    let mut keyer = ArduinoKeyer::new(Box::new(serial_io), keying_event_tx);
    if (mode == Mode::KeyerDiag) {
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
    if (mode == Mode::SourceEncoderDiag) {

    }

    //Ok(0)
    Err("message goes here".into())
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

