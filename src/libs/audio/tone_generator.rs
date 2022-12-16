
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
use std::thread;
use std::error::Error;
use std::f32::consts::PI;
use std::fmt::{Debug, Display, Formatter};
use std::sync::{Arc, Mutex, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use bus::BusReader;
use log::{debug, info, warn};
use portaudio::{NonBlocking, Output, OutputStreamSettings, PortAudio, Stream};
use portaudio as pa;

use crate::libs::application::application::BusInput;
use crate::libs::keyer_io::keyer_io::KeyingEvent;

const TABLE_SIZE: usize = 256;
// The "Radio Today guide to the Yaesu FTDX10" by Andrew Barron ZL3DW says, p. 139:
// "CW wave shape sets the shape of the CW waveform (keying envelopen rise and fall timefs). The
// default setting is 6ms. Selecting a slower rise time will make your signal sound softer. Choosing
// the faster 4ms rise time will make your signal sound a little harsher. It should only be selected
// if you are using high-speed CW."
const AMPLITUDE_DELTA: f32 = 0.005; // TODO What does this delta represent, as a rise time?

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

impl Display for AmplitudeRamping {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
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
    dt: f32, // Reciprocal of the sample rate
    thread_handle: Option<JoinHandle<()>>,
    stream: Option<Stream<NonBlocking, Output<f32>>>,
    callback_data: Arc<RwLock<Vec<Mutex<CallbackData>>>>,
    // Shared between thread and ToneGenerator
    input_rx: Arc<Mutex<Option<Arc<Mutex<BusReader<KeyingEventToneChannel>>>>>>,
}

#[derive(Clone)]
pub struct CallbackData {
    ramping: AmplitudeRamping,
    amplitude: f32, // used for ramping up/down output waveform for key click suppression
    audio_frequency: u16,
    enabled: bool,
    delta_phase: f32, // added to the phase after recording each sample
    phase: f32,       // sin(phase) is the sample value
}

impl BusInput<KeyingEventToneChannel> for ToneGenerator {
    fn clear_input_rx(&mut self) {
        match self.input_rx.lock() {
            Ok(mut locked) => { *locked = None; }
            Err(_) => {}
        }
    }

    fn set_input_rx(&mut self, input_tx: Arc<Mutex<BusReader<KeyingEventToneChannel>>>) {
        match self.input_rx.lock() {
            Ok(mut locked) => { *locked = Some(input_tx); }
            Err(_) => {}
        }
    }
}

impl ToneGenerator {
    // TODO the sidetone_audio_frequency passed into the constructor sets the callback data, but the
    // delta_phase isn't set. You have to call set_audio_frequency.
    pub fn new(sidetone_audio_frequency: u16,
               terminate: Arc<AtomicBool>) -> Self {
        // Share this holder between the ToneGenerator and its thread
        let input_rx_holder: Arc<Mutex<Option<Arc<Mutex<BusReader<KeyingEventToneChannel>>>>>> = Arc::new(Mutex::new(None));
        let move_clone_input_rx_holder = input_rx_holder.clone();

        info!("Initialising Tone generator");
        let sidetone_callback_data = CallbackData {
            ramping: AmplitudeRamping::Stable,
            amplitude: 0.0,
            audio_frequency: sidetone_audio_frequency,
            enabled: true, // cannot be disabled
            delta_phase: 0.0,
            phase: 0.0,
        };
        // TODO replace this Mutex with atomics to reduce contention in the callback.
        let arc_lock_sidetone_callback_data = Arc::new(RwLock::new(vec![Mutex::new(sidetone_callback_data)]));
        let move_clone_sidetone_callback_data = arc_lock_sidetone_callback_data.clone();
        Self {
            input_rx: input_rx_holder,
            enabled_in_filter_bandpass: true,
            sample_rate: 0, // will be initialised when the callback is initialised
            dt: 0.0,        // will be initialised when the callback is initialised
            thread_handle: Some(thread::spawn(move || {
                info!("Tone generator keying listener thread started");
                loop {
                    if terminate.load(Ordering::SeqCst) {
                        info!("Terminating tone generator thread");
                        break;
                    }

                    // Can be updated by the BusInput<KeyingEventToneChannel> above
                    let mut need_sleep = false;
                    match move_clone_input_rx_holder.lock().unwrap().as_deref() {
                        None => {
                            // Input channel hasn't been set yet; sleep after releasing lock
                            need_sleep = true;
                        }
                        Some(input_rx) => {
                            match input_rx.lock().unwrap().recv_timeout(Duration::from_millis(50)) {
                                Ok(keying_event_tone_channel) => {
                                    info!("Tone generator got {:?}", keying_event_tone_channel);
                                    if keying_event_tone_channel.tone_channel >= move_clone_sidetone_callback_data.read().unwrap().len() {
                                        warn!("Incoming tone channel {} not in use", keying_event_tone_channel.tone_channel);
                                    } else {
                                        let callback_datas = move_clone_sidetone_callback_data.read().unwrap();
                                        let mut locked_callback_data =  callback_datas[keying_event_tone_channel.tone_channel].lock().unwrap();
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
                    }
                    if need_sleep {
                        thread::sleep(Duration::from_millis(100));
                    }
                }
                debug!("Tone generator keying listener thread stopped");
            })),
            callback_data: arc_lock_sidetone_callback_data,
            stream: None,
        }
    }

    // The odd form of this callback setup (pass in the PortAudio and settings) rather than just
    // returning the callback to the caller to do stuff with... is because I can't work out what
    // the correct type signature of a callback-returning function should be.
    pub fn start_callback(&mut self, pa: &PortAudio, mut output_settings: OutputStreamSettings<f32>) -> Result<(), Box<dyn Error>> {
        let sample_rate = output_settings.sample_rate as u32;
        self.sample_rate = sample_rate;
        self.dt = 1.0_f32 / (sample_rate as f32);
        debug!("sample rate is {}",sample_rate);
        self.set_delta_phase(0);

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
                let callback_datas = move_clone_callback_data.read().unwrap();
                for tone in &*callback_datas {
                    let mut locked_callback_data = tone.lock().unwrap();
                    // TODO: Use a cosine ramping rather than this linear one?
                    match locked_callback_data.ramping {
                        AmplitudeRamping::RampingUp => {
                            if locked_callback_data.amplitude <= 0.0 {
                                locked_callback_data.amplitude = 0.0;
                                locked_callback_data.phase = 0.0;
                            }
                            if locked_callback_data.amplitude < 0.95 {
                                locked_callback_data.amplitude += AMPLITUDE_DELTA;
                            } else {
                                locked_callback_data.amplitude = 0.95;
                                locked_callback_data.ramping = AmplitudeRamping::Stable;
                            }
                        }
                        AmplitudeRamping::RampingDown => {
                            locked_callback_data.amplitude -= AMPLITUDE_DELTA;
                            if locked_callback_data.amplitude <= 0.0 {
                                locked_callback_data.amplitude = 0.0;
                                locked_callback_data.ramping = AmplitudeRamping::Stable;
                                locked_callback_data.phase = 0.0;
                            }
                        }
                        AmplitudeRamping::Stable => {
                            // noop
                        }
                    }

                    locked_callback_data.phase += locked_callback_data.delta_phase;
                    let sine_val = f32::sin(locked_callback_data.phase) * locked_callback_data.amplitude;

                    drop(locked_callback_data);

                    total_sine_val += sine_val;
                }
                total_sine_val /= callback_datas.len() as f32;

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
        {
            let callback_datas = self.callback_data.read().unwrap();
            if tone_index >= callback_datas.len() {
                return;
            }
            let mut locked_callback_data = callback_datas[tone_index].lock().unwrap();
            locked_callback_data.audio_frequency = freq;
        }
        self.set_delta_phase(tone_index);
    }

    fn set_delta_phase(&mut self, tone_index: usize) {
        let callback_datas = self.callback_data.read().unwrap();
        if tone_index >= callback_datas.len() {
            return;
        }
        // TODO ew this stinks
        if self.sample_rate == 0 {
            debug!("Sample rate not yet set; will set frequency when this is known");
            return;
        }
        let mut locked_callback_data = callback_datas[tone_index].lock().unwrap();
        locked_callback_data.delta_phase = 2.0_f32 * PI * (locked_callback_data.audio_frequency as f32) / (self.sample_rate as f32);
        debug!("Setting tone#{} frequency to {}, sample_rate {}", tone_index, locked_callback_data.audio_frequency, self.sample_rate);
    }

    pub fn set_in_filter_bandpass(&mut self, in_bandpass: bool) -> () {
        self.enabled_in_filter_bandpass = in_bandpass;
    }

    // Allocate the first disabled channel, or extend if there isn't one.
    pub fn allocate_channel(&mut self, freq: u16) -> usize {
        let callback_data = CallbackData {
            ramping: AmplitudeRamping::Stable,
            amplitude: 0.0,
            audio_frequency: freq,
            enabled: true, // well if you're allocating it, it's enabled!
            delta_phase: 0.0,
            phase: 0.0,
        };
        let mut callback_datas = self.callback_data.write().unwrap();
        // Ignore channel 0, the sidetone
        let mut tone_index = 0;
        for i in 1..callback_datas.len() {
            if callback_datas[i].lock().unwrap().enabled == false {
                callback_datas[i] = Mutex::new(callback_data.clone());
                debug!("Allocating disabled channel {}", i);
                tone_index = i;
                break
            }
        }
        if tone_index == 0 {
            tone_index = callback_datas.len();
            debug!("Allocating new channel {}", tone_index);
            callback_datas.push(Mutex::new(callback_data.clone()));
        }
        // Nothing disabled, so add..
        drop(callback_datas);
        self.set_delta_phase(tone_index);
        tone_index
    }

    // Set a channel to disabled; if it is the last channel, pop it (and all disabled at the end)
    pub fn deallocate_channel(&mut self, tone_index: usize) {
        // Tone index 0 is for the sidetone; it cannot be deallocated.
        if tone_index == 0 {
            return;
        }
        let mut callback_datas = self.callback_data.write().unwrap();
        if tone_index >= callback_datas.len() {
            return;
        }
        {
            callback_datas[tone_index].lock().unwrap().enabled = false;
        }
        while callback_datas.len() > 1 && callback_datas.last().unwrap().lock().unwrap().enabled == false {
            callback_datas.pop();
        }
    }

    // Used by tests to check allocate/deallocate functions.
    #[cfg(test)]
    pub fn test_get_enabled_states(&mut self) -> Vec<bool> {
        let callback_datas = self.callback_data.write().unwrap();
        let mut out = Vec::with_capacity(callback_datas.len());
        for callback_data in &*callback_datas {
            let locked_callback_data = callback_data.lock().unwrap();
            out.push(locked_callback_data.enabled);
        }
        out
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
#[cfg(test)]
#[path = "./tone_generator_channel_alloc_spec.rs"]
mod tone_generator_channel_alloc_spec;
