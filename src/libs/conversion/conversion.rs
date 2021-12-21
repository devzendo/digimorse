use log::debug;
use crate::libs::keyer_io::keyer_io::{KeyerEdgeDurationMs, KeyingEvent, KeyingTimedEvent};

// Prosigns not supported
pub fn char_to_morse(ch: char) -> String {
    let upper = ch.to_ascii_uppercase();
    let str = match upper {
        'A' => { ".-" }
        'B' => { "-..." }
        'C' => { "-.-." }
        'D' => { "-.." }
        'E' => { "." }
        'F' => { "..-." }
        'G' => { "--." }
        'H' => { "...." }
        'I' => { ".." }
        'J' => { ".---" }
        'K' => { "-.-" }
        'L' => { ".-.." }
        'M' => { "--" }
        'N' => { "-." }
        'O' => { "---" }
        'P' => { ".--." }
        'Q' => { "--.-" }
        'R' => { ".-." }
        'S' => { "..." }
        'T' => { "-" }
        'U' => { "..-" }
        'V' => { "...-" }
        'W' => { ".--" }
        'X' => { "-..-" }
        'Y' => { "-.--" }
        'Z' => { "--.." }
        '0' => { "-----" }
        '1' => { ".----" }
        '2' => { "..---" }
        '3' => { "...--" }
        '4' => { "....-" }
        '5' => { "....." }
        '6' => { "-...." }
        '7' => { "--..." }
        '8' => { "---.." }
        '9' => { "----." }
        '.' => { ".-.-.-" }
        '/' => { "-..-." }
        ',' => { "--..--" }
        '?' => { "..--.." }
        ' ' => { " " }
        // My shorthand
        '=' => { "-...-" }  // BT
        '|' => { "...-.-" } // SK
        '+' => { ".-.-." }  // AR
        _ => {
            panic!("Unknown character input '{}'", upper);
        }
    };
    str.to_owned()
}

// Prosigns not supported
pub fn text_to_keying(wpm: u32, text: &str) -> Vec<KeyingEvent> {
    let dit = 1200 / wpm as KeyerEdgeDurationMs;
    let dah = dit * 3 as KeyerEdgeDurationMs;
    let wordgap = dit * 7 as KeyerEdgeDurationMs;
    let text_len = text.len();

    let mut out: Vec<KeyingEvent> = Vec::new();
    out.push(KeyingEvent::Start());

    let mut up = true;
    let mut previous_char = '~'; // should never occur in input
    for (index, ch) in text.chars().enumerate() {
        if previous_char == ' ' {
            debug!("Adding word gap");
            out.push(KeyingEvent::Timed(KeyingTimedEvent{ up, duration: wordgap }));
            up = !up;
        } else {
            if ch != ' ' && index != 0 {
                debug!("Adding inter-character dah");
                out.push(KeyingEvent::Timed(KeyingTimedEvent{ up, duration: dah }));
                up = !up;
            }
        }
        if ch != ' ' {
            let morse_string = char_to_morse(ch);
            debug!("Converted '{}' to '{}'", ch, morse_string);
            for (dds_index, dot_dash_space) in morse_string.chars().enumerate() {
                let last_dds = dds_index == morse_string.len() - 1;
                debug!("Converting '{}'", dot_dash_space);
                match dot_dash_space {
                    '.' => {
                        out.push(KeyingEvent::Timed(KeyingTimedEvent{ up, duration: dit }));
                    }
                    '-' => {
                        out.push(KeyingEvent::Timed(KeyingTimedEvent{ up, duration: dah }));
                    }
                    _ => { panic!("Won't get here") }
                }
                up = !up;
                if !last_dds {
                    debug!("Adding inter-element dit");
                    out.push(KeyingEvent::Timed(KeyingTimedEvent{ up, duration: dit }));
                    up = !up;
                }
            }
        }
        previous_char = ch;
    }

    out.push(KeyingEvent::End());
    out
}


#[cfg(test)]
#[path = "./conversion_spec.rs"]
mod conversion_spec;
