pub fn printable(ch: u8) -> String {
    let chr = ch as char;
    return format!("0x{:02x} {}", ch, if chr.is_ascii_alphanumeric() || chr.is_ascii_punctuation() { chr } else { '.' });
}
