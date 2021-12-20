
/* Bill Somerville on the WSJT-X mailing list says, on sample rates:
   "WSJT-X requests a 48 kHz 16-bit audio stream for input and it generates output in the same
   format. The reason we suggest you use 48 kHz as the default sample rate is because operating
   system re-sampling is prone to audio artefacts that can degrade the receive audio performance.
   We actually re-sample in WSJT-X down to 12 kHz before the DSP processing which gives us a
   bandwidth of up to 6 kHz, the down sampling in WSJT-X uses a high quality algorithm but it is
   always better to do integral factor re-sampling so an input sample rate that is an exact power
   of two of the requested rate is most efficient."
 */
// Thanks to BartMassey's PortAudio-rs examples at https://github.com/BartMassey/portaudio-rs-demos

use core::fmt;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use portaudio::{NonBlocking, Output, OutputStreamSettings, PortAudio, Stream};
use portaudio as pa;
use log::{debug, info, warn};
use crate::libs::keyer_io::keyer_io::KeyingEvent;
use std::thread::JoinHandle;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use bus::BusReader;

// The ToneGenerator uses a DDS approach, as found at
// http://interface.khm.de/index.php/lab/interfaces-advanced/arduino-dds-sinewave-generator/
// and
// http://www.analog.com/static/imported-files/tutorials/MT-085.pdf
// (MT-085: Fundamentals of Direct Digital Synthesis (DDS))

const TABLE_SIZE: usize = 256;
const AMPLITUDE_DELTA: f32 = 0.005;
const TWO_TO_THIRTYTWO: usize = 2u64.pow(32) as usize;

static SINE_256:[u8; TABLE_SIZE] = [
    127,130,133,136,139,143,146,149,152,155,158,161,164,167,170,173,176,178,181,184,187,190,192,195,198,200,203,205,208,210,212,215,217,219,221,223,225,227,229,231,233,234,236,238,239,240,
    242,243,244,245,247,248,249,249,250,251,252,252,253,253,253,254,254,254,254,254,254,254,253,253,253,252,252,251,250,249,249,248,247,245,244,243,242,240,239,238,236,234,233,231,229,227,225,223,
    221,219,217,215,212,210,208,205,203,200,198,195,192,190,187,184,181,178,176,173,170,167,164,161,158,155,152,149,146,143,139,136,133,130,127,124,121,118,115,111,108,105,102,99,96,93,90,87,84,81,78,
    76,73,70,67,64,62,59,56,54,51,49,46,44,42,39,37,35,33,31,29,27,25,23,21,20,18,16,15,14,12,11,10,9,7,6,5,5,4,3,2,2,1,1,1,0,0,0,0,0,0,0,1,1,1,2,2,3,4,5,5,6,7,9,10,11,12,14,15,16,18,20,21,23,25,27,29,31,
    33,35,37,39,42,44,46,49,51,54,56,59,62,64,67,70,73,76,78,81,84,87,90,93,96,99,102,105,108,111,115,118,121,124,
];

/// A ToneChannel is an index into the ToneGenerator's tones - 0 is used for the sidetone; 1.. are
/// used for decoded/played-back streams of keying.
pub type ToneChannel = usize;

/// Incoming KeyingEvents to the ToneGenerator are augmented with their ToneChannel. This will cause
/// them to play at the frequency set for the sidetone (ToneChannel 0), or the audio offset of the
/// decoded/played-back received stream of keying.
#[derive(Clone, PartialEq)]
pub struct KeyingEventToneChannel {
    pub keying_event: KeyingEvent,
    pub tone_channel: ToneChannel,
}

impl Display for KeyingEventToneChannel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}: #{}", self.keying_event, self.tone_channel)
    }
}

impl Debug for KeyingEventToneChannel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}: #{}", self.keying_event, self.tone_channel)
    }
}

#[derive(Clone)]
enum AmplitudeRamping {
    RampingUp, RampingDown, Stable
}

impl fmt::Display for AmplitudeRamping {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            AmplitudeRamping::RampingUp => write!(f, "^"),
            AmplitudeRamping::RampingDown => write!(f, "v"),
            AmplitudeRamping::Stable => write!(f, "-"),
        }
    }
}

// The keyer sidetone and all received, decoded streams are given a ToneGenerator each. The keyer
// sends its KeyingEvents in real-time down the keying_events channel; these are directly used to
// set the ramping appropriately. This is used in the callback to set the amplitude of the output
// waveform, which is then stored in the output buffer. Each decoded stream is handled similarly,
// with differing audio_frequency as decoded streams are played into the keying_events channel by
// the receiver playback system.
pub struct ToneGenerator {
    enabled_in_filter_bandpass: bool,
    sample_rate: u32,
    thread_handle: Option<JoinHandle<()>>,
    stream: Option<Stream<NonBlocking, Output<f32>>>,
    callback_data: Arc<Vec<Mutex<CallbackData>>>,
}

pub struct CallbackData {
    ramping: AmplitudeRamping,
    phase_accumulator: usize,
    timing_word_m: usize,
    amplitude: f32, // used for ramping up/down output waveform for key click suppression
    audio_frequency: u16,
}

impl ToneGenerator {
    pub fn new(audio_frequency: u16, mut keying_events_with_tone_channels: BusReader<KeyingEventToneChannel>, terminate:
    Arc<AtomicBool>) -> Self {
        info!("Initialising Tone generator");
        let callback_data = CallbackData {
            ramping: AmplitudeRamping::Stable,
            phase_accumulator: 0,
            timing_word_m: 0,
            amplitude: 0.0,
            audio_frequency,
        };
        // TODO replace this Mutex with atomics to reduce contention in the callback.
        let arc_lock_callback_data = Arc::new(vec![Mutex::new(callback_data)]);
        let move_clone_callback_data = arc_lock_callback_data.clone();
        Self {
            enabled_in_filter_bandpass: true,
            sample_rate: 0, // will be initialised when the callback is initialised
            thread_handle: Some(thread::spawn(move || {
                info!("Tone generator keying listener thread started");
                loop {
                    if terminate.load(Ordering::SeqCst) {
                        info!("Terminating tone generator thread");
                        break;
                    }

                    match keying_events_with_tone_channels.recv_timeout(Duration::from_millis(50)) {
                        Ok(keying_event_tone_channel) => {
                            info!("Tone generator got {:?}", keying_event_tone_channel);
                            if keying_event_tone_channel.tone_channel >= move_clone_callback_data.len() {
                                warn!("Incoming tone channel {} not in use", keying_event_tone_channel.tone_channel);
                            } else {
                                let mut locked_callback_data =  move_clone_callback_data[keying_event_tone_channel.tone_channel].lock().unwrap();
                                locked_callback_data.ramping = match keying_event_tone_channel.keying_event {
                                    KeyingEvent::Timed(event) => {
                                        if event.up {
                                            AmplitudeRamping::RampingDown
                                        } else {
                                            AmplitudeRamping::RampingUp
                                        }
                                    }
                                    KeyingEvent::Start() => {
                                        AmplitudeRamping::RampingUp
                                    }
                                    KeyingEvent::End() => {
                                        AmplitudeRamping::RampingDown
                                    }
                                };
                                // info!("Set ramping of tone channel {} to {}", keying_event.tone_channel, locked_callback_data.ramping);
                            }
                        }
                        Err(_) => {
                            // could timeout, or be disconnected?
                            // ignore for now...
                        }
                    }
                }
                debug!("Tone generator keying listener thread stopped");
            })),
            callback_data: arc_lock_callback_data,
            stream: None,
        }
    }

    // The odd form of this callback setup (pass in the PortAudio and settings) rather than just
    // returning the callback to the caller to do stuff with... is because I can't work out what
    // the correct type signature of a callback-returning function should be.
    pub fn start_callback(&mut self, pa: &PortAudio, mut output_settings: OutputStreamSettings<f32>) -> Result<(), Box<dyn Error>> {
        let sample_rate = output_settings.sample_rate as u32;
        self.sample_rate = sample_rate;
        debug!("sample rate is {}",sample_rate);
        self.set_timing_word(0);

        let move_clone_callback_data = self.callback_data.clone();
        let callback = move |pa::OutputStreamCallbackArgs::<f32> { buffer, frames, .. }| {
            // info!("buffer length is {}, frames is {}", buffer.len(), frames);
            // buffer length is 128, frames is 64; idx goes from [0..128).
            // One frame is a pair of left/right channel samples.
            // 48000/64=750 so in one second there are 48000 samples (frames), and 750 calls to this callback.
            // 1000/750=1.33333 so each buffer has a duration of 1.33333ms.
            // The fastest dit we want to encode (at 60WPM) is 20ms long.

            let mut idx = 0;

            for _ in 0..frames {
                // The processing of amplitude/phase/ramping needs to be done every frame.
                let mut total_sine_val = 0.0;
                for tone in &*move_clone_callback_data {
                    let mut locked_callback_data = tone.lock().unwrap();
                    match locked_callback_data.ramping {
                        AmplitudeRamping::RampingUp => {
                            if locked_callback_data.amplitude == 0.0 {
                                locked_callback_data.phase_accumulator = 0;
                            }
                            locked_callback_data.amplitude += AMPLITUDE_DELTA;
                            if locked_callback_data.amplitude >= 0.95 {
                                locked_callback_data.amplitude = 0.95;
                                locked_callback_data.ramping = AmplitudeRamping::Stable;
                            }
                        }
                        AmplitudeRamping::RampingDown => {
                            locked_callback_data.amplitude -= AMPLITUDE_DELTA;
                            if locked_callback_data.amplitude <= 0.0 {
                                locked_callback_data.amplitude = 0.0;
                                locked_callback_data.ramping = AmplitudeRamping::Stable;
                                locked_callback_data.phase_accumulator = 0;
                            }
                        }
                        AmplitudeRamping::Stable => {
                            // noop
                        }
                    }

                    locked_callback_data.phase_accumulator += locked_callback_data.timing_word_m;
                    let icnt= (locked_callback_data.phase_accumulator >> 24) % TABLE_SIZE;
                    //debug!("phase accumulator {} icnt {}", locked_callback_data.phase_accumulator, icnt);

                    // Original sine table was from [-1 .. 1], whereas SINE_256 is from [0 .. 255]
                    let sine_byte = SINE_256[icnt];
                    let sine_float = ((sine_byte as i16 - 127) as f32) / 127.0;
                    let sine_val = sine_float * locked_callback_data.amplitude;

                    std::mem::drop(locked_callback_data);

                    total_sine_val += sine_val;
                }
                total_sine_val /= move_clone_callback_data.len() as f32;

                // TODO MONO - if opening the stream with a single channel causes the same values to
                // be written to both left and right outputs, this could be optimised..
                buffer[idx] = total_sine_val;
                buffer[idx + 1] = total_sine_val;

                idx += 2;
            }
            // idx is 128...
            pa::Continue
        };

        // we won't output out of range samples so don't bother clipping them.
        output_settings.flags = pa::stream_flags::CLIP_OFF;

        let maybe_stream = pa.open_non_blocking_stream(output_settings, callback);
        match maybe_stream {
            Ok(mut stream) => {
                stream.start()?;
                self.stream = Some(stream);
            }
            Err(e) => {
                warn!("Error opening tone generator output stream: {}", e);
            }
        }
        Ok(())
        // Now it's playing...
    }

    pub fn set_audio_frequency(&mut self, tone_index: usize, freq: u16) -> () {
        if tone_index >= self.callback_data.len() {
            return;
        }
        {
            let mut locked_callback_data = self.callback_data[tone_index].lock().unwrap();
            locked_callback_data.audio_frequency = freq;
        }
        self.set_timing_word(tone_index);
    }

    fn set_timing_word(&mut self, tone_index: usize) {
        if tone_index >= self.callback_data.len() {
            return;
        }
        // TODO ew this stinks
        if self.sample_rate == 0 {
            debug!("Sample rate not yet set; will set frequency when this is known");
            return;
        }
        let mut locked_callback_data = self.callback_data[tone_index].lock().unwrap();
        locked_callback_data.timing_word_m = (TWO_TO_THIRTYTWO * (locked_callback_data.audio_frequency as usize) / self.sample_rate as usize) as usize;
        debug!("Setting tone#{} frequency to {}, timing_word_m {}, sample_rate {}", tone_index, locked_callback_data.audio_frequency, locked_callback_data.timing_word_m, self.sample_rate);
    }

    pub fn set_in_filter_bandpass(&mut self, in_bandpass: bool) -> () {
        self.enabled_in_filter_bandpass = in_bandpass;
    }

    pub fn allocate_channel(&mut self, freq: u16) -> usize {
        1
    }
}

impl Drop for ToneGenerator {
    fn drop(&mut self) {
        debug!("ToneGenerator stopping stream...");
        self.stream.take().map(|mut r| r.stop());
        debug!("ToneGenerator joining thread handle...");
        self.thread_handle.take().map(JoinHandle::join);
        debug!("ToneGenerator ...joined thread handle");
    }
}



#[cfg(test)]
#[path = "./tone_generator_spec.rs"]
mod tone_generator_spec;
