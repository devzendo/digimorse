
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
use portaudio::{NonBlocking, Output, OutputStreamSettings, PortAudio, Stream};
use portaudio as pa;
use log::{debug, info, warn};
use crate::libs::keyer_io::keyer_io::KeyingEvent;
use std::f64::consts::PI;
use std::thread::JoinHandle;
use std::thread;
use std::sync::{Arc, RwLock};


const TABLE_SIZE: usize = 200;
const AMPLITUDE_DELTA: f32 = 0.01;

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
    audio_frequency: u16,
    thread_handle: Option<JoinHandle<()>>,
    stream: Option<Stream<NonBlocking, Output<f32>>>, // don't know why f32
    callback_data: Arc<RwLock<CallbackData>>,
}

pub struct CallbackData {
    ramping: AmplitudeRamping,
}
impl ToneGenerator {
    pub fn new(audio_frequency: u16, keying_events: crossbeam_channel::Receiver<KeyingEvent>) -> Self {
        info!("Initialising Tone generator");
        let callback_data = CallbackData {
            ramping: AmplitudeRamping::Stable,
        };
        // TODO replace this RwLock with atomics to reduce contention in the callback.
        let arc_lock_callback_data = Arc::new(RwLock::new(callback_data));
        let move_clone_callback_data = arc_lock_callback_data.clone();
        Self {
            enabled_in_filter_bandpass: true,
            audio_frequency,
            thread_handle: Some(thread::spawn(move || {
                info!("Tone generator thread started");
                // TODO until poisoned?
                loop {
                    match keying_events.try_recv() { // should this be a timeout?
                        Ok(keying_event) => {
                            info!("Tone generator got {}", keying_event);
                            let mut locked_callback_data = move_clone_callback_data.write().unwrap();
                            locked_callback_data.ramping = match keying_event {
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
                            info!("Set ramping to {}", locked_callback_data.ramping);
                        }
                        Err(_) => {
                            // could timeout, or be disconnected?
                            // ignore for now...
                        }
                    }
                }
                // TODO when we swallow poison, exit here.
                // debug!("Tone generator thread stopped");
            })),
            callback_data: arc_lock_callback_data,
            stream: None,
        }
    }

    // The odd form of this callback setup (pass in the PortAudio and settings) rather than just
    // returning the callback to the caller to do stuff with... is because I can't work out what
    // the correct type signature of a callback-returning function should be.
    pub fn start_callback(&mut self, pa: &PortAudio, mut output_settings: OutputStreamSettings<f32>) -> Result<(), Box<dyn Error>> {
        let mut sine: [f32; TABLE_SIZE] = [0.0; TABLE_SIZE];
        for i in 0..TABLE_SIZE {
            sine[i] = (i as f64 / TABLE_SIZE as f64 * PI * 2.0).sin() as f32;
        }
        let mut phase: usize = 0;
        let mut amplitude: f32 = 0.0; // used for ramping up/down output waveform for key click suppression
        let move_clone_callback_data = self.callback_data.clone();
        let callback = move |pa::OutputStreamCallbackArgs { buffer, frames, .. }| {
            // info!("buffer length is {}, frames is {}", buffer.len(), frames);
            // buffer length is 128, frames is 64; idx goes from [0..128).
            // One frame is a pair of left/right channel samples.
            // 48000/64=750 so in one second there are 48000 samples (frames), and 750 calls to this callback.
            // 1000/750=1.33333 so each buffer has a duration of 1.33333ms.

            let mut idx = 0;

            for _ in 0..frames {
                // Would it be precise enough to adjust the direction of ramping every frame?
                let mut locked_callback_data = move_clone_callback_data.write().unwrap();
                match locked_callback_data.ramping {
                    AmplitudeRamping::RampingUp => {
                        amplitude += AMPLITUDE_DELTA;
                        if amplitude >= 1.0 {
                            amplitude = 1.0;
                            locked_callback_data.ramping = AmplitudeRamping::Stable;
                        }
                    }
                    AmplitudeRamping::RampingDown => {
                        amplitude -= AMPLITUDE_DELTA;
                        if amplitude <= 0.0 {
                            amplitude = 0.0;
                            locked_callback_data.ramping = AmplitudeRamping::Stable;
                        }
                    }
                    AmplitudeRamping::Stable => {
                        // noop
                    }
                }
                std::mem::drop(locked_callback_data);

                let sine_val = sine[phase] * amplitude;
                // TODO MONO - if opening the stream with a single channel causes the same values to
                // be written to both left and right outputs, this could be optimised..
                buffer[idx] = sine_val;
                buffer[idx + 1] = sine_val;
                phase += 1;
                if phase >= TABLE_SIZE {
                    phase -= TABLE_SIZE;
                }
                idx += 2;
            }
            // idx is 128...
            pa::Continue
        };

        // we won't output out of range samples so don't bother clipping them.
        output_settings.flags = pa::stream_flags::CLIP_OFF;
        debug!("output settings: {:?}", output_settings);

        let maybe_stream = pa.open_non_blocking_stream(output_settings, callback);
        match maybe_stream {
            Ok(mut stream) => {
                info!("Opened tone generator output stream ok");
                stream.start()?;
                self.stream = Some(stream);
                info!("Started tone generator output stream ok");
            }
            Err(e) => {
                warn!("Error opening tone generator output stream: {}", e);
            }
        }
        info!("Got to the end of tone generator callback setup");
        Ok(())
        // Now it's playing...
    }

    pub fn set_audio_frequency(&mut self, freq: u16) -> () {
        self.audio_frequency = freq;
    }

    pub fn set_in_filter_bandpass(&mut self, in_bandpass: bool) -> () {
        self.enabled_in_filter_bandpass = in_bandpass;
    }
}

impl Drop for ToneGenerator {
    fn drop(&mut self) {
        debug!("ToneGenerator joining thread handle...");
        self.thread_handle.take().map(JoinHandle::join);
        debug!("ToneGenerator ...joined thread handle");
    }
}
