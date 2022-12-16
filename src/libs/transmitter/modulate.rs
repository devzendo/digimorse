// Based on Kārlis Goba's GFSK modulation of FT8 symbols, from
// https://github.com/kgoba/ft8_lib/blob/master/gen_ft8.c

use crate::libs::channel_codec::channel_encoding::ChannelSymbol;
use crate::libs::transmitter::transmitter::AudioFrequencyHz;

const SYMBOL_PERIOD_SECONDS: f32 = 0.160_f32;

// Synthesize a waveform of tones, based on the channel_symbols, at the sample_rate, with a base
// audio frequency given by the audio_offset. Shape the tones using Gaussian Frequency Shift Keying
// phase shaping. Store the waveform in the supplied array of signal waveform samples.
// There should be sample_rate * channel_symbols.len() * SYMBOL_PERIOD_SECONDS samples.
// If channel_symbols starts or ends with RampUp/RampDown symbols, the amplitude of the start/end
// of the waveform will be ramped accordingly.
// Note: It is the caller's responsibility to decide whether to ramp up / down, and to provide
// a waveform_store large enough to store the whole modulated channel_symbols.
pub fn gfsk_modulate(_audio_offset: AudioFrequencyHz, sample_rate: AudioFrequencyHz, channel_symbols: &Vec<ChannelSymbol>, waveform_store: &[f32]) -> () {
    // Sample rate is 48000Hz.
    let expected_waveform_store_len = (sample_rate as f32 * channel_symbols.len() as f32 * SYMBOL_PERIOD_SECONDS) as usize;
    if waveform_store.len() != expected_waveform_store_len {
        panic!("Cannot store gfsk_modulate waveform in {} f32s, expecting {}", waveform_store.len(), expected_waveform_store_len);
    }
}

#[cfg(test)]
#[path = "modulate_spec.rs"]
mod modulate_spec;
