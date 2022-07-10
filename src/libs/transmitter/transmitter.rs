

pub type RadioFrequencyMHz = u32;
pub type AudioFrequencyKHz = u16;

pub struct Transmitter {
    _radio_frequency_mhz: RadioFrequencyMHz,
    _audio_offset: AudioFrequencyKHz,
}

impl Transmitter {
    pub fn new() -> Self {
        Self {
            _radio_frequency_mhz: 0,
            _audio_offset: 0
        }
    }
}

// Two implementations: an audio version for driving a traditional transmitter, and a direct version
// that drives a DDS chip.

#[cfg(test)]
#[path = "./transmitter_spec.rs"]
mod transmitter_spec;
