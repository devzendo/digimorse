
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

use std::error::Error;
use portaudio::{NonBlocking, Output, OutputStreamSettings, PortAudio, Stream};
use portaudio as pa;
use log::debug;
use std::sync::mpsc::Receiver;
use crate::libs::keyer_io::keyer_io::KeyingEvent;
use std::f64::consts::PI;
use std::thread::JoinHandle;
use std::thread;
use std::sync::{Arc, RwLock};


const TABLE_SIZE: usize = 200;

enum AmplitudeRamping {
    RampingUp, RampingDown, Stable
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
    sine: [f32; TABLE_SIZE],
    ramping: Arc<RwLock<AmplitudeRamping>>,
    thread_handle: Option<JoinHandle<()>>,
    stream: Option<Stream<NonBlocking, Output<i16>>>,
}

impl ToneGenerator {
    pub fn new(audio_frequency: u16, keying_events: crossbeam_channel::Receiver<KeyingEvent>) -> Self {
        let mut sine: [f32; TABLE_SIZE] = [0.0; TABLE_SIZE];
        for i in 0..TABLE_SIZE {
            sine[i] = (i as f64 / TABLE_SIZE as f64 * PI * 2.0).sin() as f32;
        }
        // TODO replace this RwLock with atomics to reduce contention in the callback.
        let ramping = Arc::new(RwLock::new(AmplitudeRamping::Stable));
        let move_clone_ramping = ramping.clone();
        Self {
            enabled_in_filter_bandpass: true,
            audio_frequency,
            sine,
            ramping,
            thread_handle: Some(thread::spawn(move || {
                let mut amplitude: f32 = 0.0; // used for ramping up/down output waveform for key click suppression

                debug!("Tone generator thread started");
                // TODO until poisoned?
                loop {
                    match keying_events.try_recv() { // should this be a timeout?
                        Ok(keying_event) => {
                            *(move_clone_ramping.write().unwrap()) = match keying_event {
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
                            }
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
            stream: None,
        }
    }

    // The odd form of this callback setup (pass in the PortAudio and settings) rather than just
    // returning the callback to the caller to do stuff with... is because I can't work out what
    // the correct type signature of a callback-returning function should be.
    pub fn start_callback(&mut self, pa: &PortAudio, output_settings: OutputStreamSettings<i16>) -> Result<(), Box<dyn Error>> {
        /*
        let rrrr = move |pa::OutputStreamCallbackArgs { buffer, frames, .. }| {
            let mut idx = 0;
            for _ in 0..frames {
                buffer[idx] = sine[left_phase];
                buffer[idx + 1] = sine[right_phase];
                left_phase += 1;
                if left_phase >= TABLE_SIZE {
                    left_phase -= TABLE_SIZE;
                }
                right_phase += 3;
                if right_phase >= TABLE_SIZE {
                    right_phase -= TABLE_SIZE;
                }
                idx += 2;
            }
            pa::Continue
        };
        rrrr
        */
        let callback = move |pa::OutputStreamCallbackArgs { buffer, frames, .. }| {
            pa::Continue
        };

        let stream = pa.open_non_blocking_stream(output_settings, callback)?;
        self.stream = Some(stream);
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
