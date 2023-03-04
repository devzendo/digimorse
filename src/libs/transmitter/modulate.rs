// Based on Kārlis Goba's GFSK modulation of FT8 symbols, from
// https://github.com/kgoba/ft8_lib/blob/master/gen_ft8.c
// With assistance from Minoru Tomobe's RustFT8 at
// https://github.com/w-ockham/RustFT8/blob/main/src/gfsk.rs

use log::debug;
use crate::libs::channel_codec::channel_encoding::ChannelSymbol;
use crate::libs::transmitter::transmitter::AudioFrequencyHz;
use crate::libs::util::graph::plot_graph;

pub const SYMBOL_SMOOTHING_FILTER_BANDWIDTH: f32 = 2.0f32; // TODO FT8 uses 2; FT4 uses 1; unsure what to use here
pub const SYMBOL_PERIOD_SECONDS: f32 = 0.160_f32;
pub const RAMP_SYMBOL_PERIOD_SECONDS: f32 = 0.020_f32;
const PI: f32 = std::f32::consts::PI;

const GFSK_CONST_K: f32 = 5.336446f32; // PI * sqrt(2 / log(2))

// Compute a GFSK shaping pulse. The pulse is theoretically infinitely long but it's truncated at
// 3*symbol length; the pulse array only needs 3*n_spsym elements.
// samples_per_symbol: number of samples per symbol
// pulse: output pulse samples array
pub fn gfsk_pulse(samples_per_symbol: usize, pulse: &mut [f32]) {
    for (i, p) in pulse.iter_mut().enumerate().take(SYMBOL_WIDTH_IN_SPSYM * samples_per_symbol) {
        let t = i as f32 / samples_per_symbol as f32 - 1.5;
        let arg1 = GFSK_CONST_K * SYMBOL_SMOOTHING_FILTER_BANDWIDTH * (t + 0.5);
        let arg2 = GFSK_CONST_K * SYMBOL_SMOOTHING_FILTER_BANDWIDTH * (t - 0.5);
        *p = (libm::erff(arg1) - libm::erff(arg2)) / 2.0;
    }
}


const SYMBOL_WIDTH_IN_SPSYM: usize = 3;
const RAMP_SYMBOL_WIDTH_IN_SPSYM: usize = 2;  // WHY 2 * n_spsym (when the above channel symbol modulation uses 3 * n_spsym)?

/// Synthesize a waveform of tones, based on the channel_symbols, at the sample_rate, with a base
/// audio frequency given by the audio_offset. Shape the tones using Gaussian Frequency Shift Keying
/// phase shaping. Store the waveform in the supplied array of signal waveform samples.
/// There should be sample_rate * channel_symbols.len() * SYMBOL_PERIOD_SECONDS samples.
/// If channel_symbols starts or ends with RampUp/RampDown symbols, the amplitude of the start/end
/// of the waveform will be ramped accordingly.
/// Note: It is the caller's responsibility to decide whether to ramp up / down, and to provide
/// a waveform_store large enough to store the whole modulated channel_symbols.
/// @param[in] audio_offset is the base audio frequency of the synthesized waveform
/// @param[in] sample_rate is the sample rate of the output device used to emit the synthesized
/// waveform
/// @param[in] channel_symbols a vector of channel symbols; if emitting the ramp up/down waveforms
/// then the first and last channel symbol is used for these
/// @param[in] waveform_store will be mutated to contain the emitted synthesized waveform
/// @param[in] need_ramp_up indicates the start of a transmitted sequence of waveforms, and that
/// the first symbol should be repeated with a ramped-up waveform
/// @param[in] need_ramp_up indicates the final waveform in a transmission, and that the last
/// symbol should be repeated with a ramped-down waveform
/// @param[out] The return is the number of samples stored in the waveform_store, based on whether
/// the ramp up/down samples are present.
pub fn gfsk_modulate(audio_offset: AudioFrequencyHz, sample_rate: AudioFrequencyHz,
                     channel_symbols: &Vec<ChannelSymbol>, waveform_store: &mut [f32],
                     need_ramp_up: bool, need_ramp_down: bool) -> usize {
    if sample_rate == 0 {
        panic!("No sample rate defined for gfsk_modulate");
    }
    // Sample rate is 48000Hz.
    let samples_per_symbol = (sample_rate as f32 * SYMBOL_PERIOD_SECONDS) as usize; // Samples per symbol
    let n_sym = channel_symbols.len() + (if need_ramp_up { 1 } else { 0 }) + (if need_ramp_down { 1 } else { 0 });
    let total_number_of_samples = n_sym * samples_per_symbol;
    debug!("sample_rate {} # channel_symbols {} samples_per_symbol {} n_sym {} n_wave {}", sample_rate, channel_symbols.len(), samples_per_symbol, n_sym, total_number_of_samples);
    if waveform_store.len() < total_number_of_samples {
        panic!("Cannot store gfsk_modulate waveform in {} f32s, expecting {}", waveform_store.len(), total_number_of_samples);
    }
    let peak_amplitude = 1.0f32; // TODO need to take this limiting of amplitude from the transmitter.

    // Compute the smoothed frequency waveform.
    // Length = (n_sym+2)*samples_per_symbol samples, first and last symbols extended
    let dphi_peak = 2.0 * PI * peak_amplitude / samples_per_symbol as f32;
    let mut dphi = Vec::new();

    // Shift frequency up by audio_offset Hz
    let audio_offset_dphi = 2.0 * PI * audio_offset as f32 / sample_rate as f32;
    for _ in 0..(total_number_of_samples + 2 * samples_per_symbol) {
        dphi.push(audio_offset_dphi);
    }

    let mut pulse = vec![0.0; SYMBOL_WIDTH_IN_SPSYM * samples_per_symbol];

    debug!("Creating GFSK pulse");
    gfsk_pulse(samples_per_symbol, &mut pulse);

    plot_graph(
        "./gauss-envelope.png",
        "GFSK Phase Envelope",
        &pulse,
        0,
        pulse.len(),
        0.0,
        1.0,
    );

    let mut symbol_index = 0;

    // Add dummy symbol at beginning with tone value equal to 1st symbol if necessary.
    if need_ramp_up {
        let first_channel_symbol = channel_symbols[0] as f32;
        debug!("Adding ramp up symbol of #{}", first_channel_symbol);
        for j in 0..(RAMP_SYMBOL_WIDTH_IN_SPSYM * samples_per_symbol) {
            dphi[j] += dphi_peak * pulse[j + samples_per_symbol] * first_channel_symbol;
        }
        symbol_index += 1;
    }

    // Modulate the channel symbols...
    debug!("Modulating channel symbols");
    for sym in channel_symbols.as_slice().iter() {
        let ib = symbol_index * samples_per_symbol;
        //debug!("channel symbol #{} at offset {}={}", symbol_index, ib, sym);

        for j in 0..(SYMBOL_WIDTH_IN_SPSYM * samples_per_symbol) { // WHY 3 * n_spsym? (same length as the gfsk pulse)
            dphi[j + ib] += dphi_peak * pulse[j] * (*sym as f32);
            //debug!("  #{}={}", j+ib, dphi[j+ib]);
        }
        symbol_index += 1;
    }

    // Add dummy symbol at end with tone value equal to last symbol if necessary.
    if need_ramp_down {
        let ib = symbol_index * samples_per_symbol;
        let last_channel_symbol = channel_symbols[channel_symbols.len() - 1] as f32;
        debug!("Adding ramp down symbol of #{}", last_channel_symbol);
        for j in 0..(RAMP_SYMBOL_WIDTH_IN_SPSYM * samples_per_symbol) {
            dphi[j + ib] += dphi_peak * pulse[j] * last_channel_symbol
        }
    }

    debug!("plotting tones.png with {} tones", dphi.len());

    plot_graph("./tones.png", "GFSK Tones", &dphi, 0, dphi.len(), 0.07, 0.1);

    debug!("Calculating waveform");
    // Calculate and insert the audio waveform
    let mut phi = 0.0f32;
    for k in 0..total_number_of_samples {
        // Don't include dummy symbols
        waveform_store[k] = phi.sin();
        phi = libm::fmodf(phi + dphi[k + samples_per_symbol], 2.0 * PI);
    }

    // Apply envelope shaping to the first and last symbols if necessary.
    if need_ramp_up || need_ramp_down {
        debug!("Shaping envelope");
        let n_ramp = samples_per_symbol / 8;
        for i in 0..n_ramp {
            let env = (1.0 - (2.0 * PI * i as f32 / (2.0 * n_ramp as f32)).cos()) / 2.0;
            if need_ramp_up {
                waveform_store[i] *= env;
            }
            if need_ramp_down {
                waveform_store[total_number_of_samples - 1 - i] *= env;
            }
        }
    }
    debug!("Plotting ramp up / down");
    plot_graph("./ramp-up.png", "Modulated waveform", &waveform_store, 0, samples_per_symbol * 3, -1.1, 1.1);
    plot_graph("./ramp-down.png", "Modulated waveform", &waveform_store, total_number_of_samples - (samples_per_symbol * 3), total_number_of_samples, -1.1, 1.1);

    debug!("Finished modulation");
    total_number_of_samples
}

#[cfg(test)]
#[path = "modulate_spec.rs"]
mod modulate_spec;
