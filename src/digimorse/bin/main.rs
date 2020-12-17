extern crate clap;

use clap::{App, /*Arg,*/ ArgMatches};

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn parse_command_line<'a>() -> ArgMatches<'a> {
    return App::new("digimorse")
        .version(VERSION)
        .author("Matt Gumbley <matt.gumbley@gmail.com>")
        .about("Digitally Encoded Morse Transceiver")
        .get_matches();
}

fn main() {
    let _arguments = parse_command_line();
}
