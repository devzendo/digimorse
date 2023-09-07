use std::fs::File;
use std::path::Path;
use wav::{BitDepth, Header, WAV_FORMAT_IEEE_FLOAT};
use crate::libs::audio::audio_devices::SAMPLE_RATE;

pub fn write_waveform_file(sample_waveform: Vec<f32>, filename: &str) -> std::io::Result<()> {
    let mut out_file = File::create(Path::new(filename)).unwrap();
    let header = Header::new(WAV_FORMAT_IEEE_FLOAT, 1, SAMPLE_RATE as u32, 32);
    let data = BitDepth::ThirtyTwoFloat(sample_waveform);
    wav::write(header, &data, &mut out_file)
}


pub fn read_waveform_file(filename: &str) -> std::io::Result<Vec<f32>> {
    let mut in_file = File::open(Path::new(filename)).unwrap();
    let (_header, data) = wav::read(&mut in_file)?;
    Ok(data.as_thirty_two_float().unwrap().to_vec())
}

