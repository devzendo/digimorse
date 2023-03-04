use std::error::Error;
use std::f32::consts::PI;
use std::sync::{Arc, Mutex, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use bus::BusReader;
use log::{debug, info, warn};
use portaudio::{NonBlocking, Output, OutputStreamSettings, PortAudio, Stream};
use portaudio as pa;
use crate::libs::application::application::BusInput;
use crate::libs::channel_codec::channel_encoding::ChannelEncoding;
use crate::libs::source_codec::source_encoding::SOURCE_ENCODER_BLOCK_SIZE_IN_BITS;
use crate::libs::transmitter::modulate::{gfsk_modulate, RAMP_SYMBOL_PERIOD_SECONDS, SYMBOL_PERIOD_SECONDS};

pub type RadioFrequencyMHz = u32;
pub type AudioFrequencyHz = u16;
pub type AmplitudeMax = f32; // 0.0 to 1.0 to scale the output power

pub const COSTAS_ARRAY_SYMBOLS: usize = 7; // TODO: no Costas array yet

/*
 * The Transmitter receives ChannelEncodings (block of symbols and end flag) on its input bus.
 * It decides to add RampUp/RampDown symbols to these, based on whether it is currently silent (not
 * transmitting tones), and whether the end flag is set. These are then converted to a GFSK
 * waveform, in a pool-allocated buffer of samples, and passed to the audio output callback that
 * PortAudio will be calling. When that callback has finished with the sample buffer it is released
 * to the pool.
 * TODO: pool allocation - there's just one buffer for now
 */
pub struct Transmitter {
    _radio_frequency_mhz: RadioFrequencyMHz, // TODO CAT controller will need this?
    _audio_offset: AudioFrequencyHz,
    amplitude_max: AmplitudeMax,
    sample_rate: u32,
    dt: f32, // Reciprocal of the sample rate
    terminate: Arc<AtomicBool>,
    thread_handle: Option<JoinHandle<()>>,
    stream: Option<Stream<NonBlocking, Output<f32>>>,
    callback_data: Arc<RwLock<CallbackData>>,

    // Shared between thread and Transmitter
    input_rx: Arc<Mutex<Option<Arc<Mutex<BusReader<ChannelEncoding>>>>>>,
}

#[derive(Clone)]
struct CallbackData {
    _amplitude: f32, // used for ramping up/down output waveform at start and end
    audio_frequency: AudioFrequencyHz,
    amplitude_max: AmplitudeMax,
    delta_phase: f32, // added to the phase after recording each sample
    _phase: f32,       // sin(phase) is the sample value
    sample_rate: u32, // Hz
    samples: Vec<f32>, // contains the GFSK modulated waveform to emit, allocated as a Vec, used as a slice
    samples_written: usize, // contains the number of samples written to 'samples', could be <= the size of that vector
    sample_index: usize, // next sample to emit
    silent: Arc<AtomicBool>,
}

pub fn maximum_number_of_symbols() -> usize {
    // A source encoded block is SOURCE_ENCODER_BLOCK_SIZE_IN_BITS bits + 2 spare bits + 14-bit CRC = 112 + 2 + 14 = 128
    // This is then LDPC-encoded to yield 256 bits of codeword. Each byte of that (32 bytes)
    // yield 2 symbols. The maximum number of symbols transmitted is therefore:
    // Costas array and a frame of 64 symbols.
    // TODO Costas Array might not be 7 symbols ..
    let channel_encoded_bits = (SOURCE_ENCODER_BLOCK_SIZE_IN_BITS + 2 + 14) * 2;
    let channel_encoded_symbols = (channel_encoded_bits / 8) * 2;
    // Note: ramp up/down are shorter than a full symbol and aren't counted here.
    COSTAS_ARRAY_SYMBOLS + channel_encoded_symbols
}

impl Transmitter {
    pub fn new(audio_offset: AudioFrequencyHz, terminate: Arc<AtomicBool>,
               /* TODO transmit halt AtomicBool */ /* TODO CAT controller passed in here */) -> Self {
        // Share this holder between the Transmitter and its thread
        let input_rx_holder: Arc<Mutex<Option<Arc<Mutex<BusReader<ChannelEncoding>>>>>> = Arc::new(Mutex::new(None));
        let move_clone_input_rx_holder = input_rx_holder.clone();

        info!("Initialising Transmitter");
        let silent = Arc::new(AtomicBool::new(true));
        let modulation_callback_data = CallbackData {
            _amplitude: 0.0,
            audio_frequency: audio_offset,
            amplitude_max: 1.0,
            delta_phase: 0.0,
            _phase: 0.0,
            sample_rate: 0,
            samples: vec![],
            samples_written: 0,
            sample_index: 0,
            silent
        };
        // TODO replace this Mutex with atomics to reduce contention in the callback.
        let arc_lock_modulation_callback_data = Arc::new(RwLock::new(modulation_callback_data));
        let move_clone_modulation_callback_data = arc_lock_modulation_callback_data.clone();


        Self {
            _radio_frequency_mhz: 0,
            _audio_offset: 0,
            amplitude_max: 1.0,
            sample_rate: 0, // will be initialised when the callback is initialised
            dt: 0.0,        // will be initialised when the callback is initialised
            terminate: terminate.clone(),
            input_rx: input_rx_holder,    // Modified by BusInput
            thread_handle: Some(thread::spawn(move || {
                info!("Transmitter channel-encoding listener thread started");
                loop {
                    if terminate.load(Ordering::SeqCst) {
                        info!("Terminating transmitter thread");
                        break;
                    }

                    // Can be updated by the BusInput<ChannelEncoding> above
                    let mut need_sleep = false;
                    match move_clone_input_rx_holder.lock().unwrap().as_deref() {
                        None => {
                            // Input channel hasn't been set yet; sleep after releasing lock
                            need_sleep = true;
                        }
                        Some(input_rx) => {
                            match input_rx.lock().unwrap().recv_timeout(Duration::from_millis(50)) {
                                Ok(channel_encoding) => {
                                    info!("Transmitter got {:?}", channel_encoding);
                                    let mut locked_callback_data = move_clone_modulation_callback_data.write().unwrap();
                                    let need_ramp_up = locked_callback_data.silent.load(Ordering::SeqCst);
                                    let need_ramp_down = channel_encoding.is_end;
                                    debug!("Ramp up {} down {}", need_ramp_up, need_ramp_up);
                                    // Convert the channel_encoding into a GFSK waveform, and set it in the locked_callback_data
                                    // for the callback to emit.
                                    debug!("waveform store has {} space", locked_callback_data.samples.capacity());
                                    locked_callback_data.samples_written = gfsk_modulate(locked_callback_data.audio_frequency,
                                                                                         locked_callback_data.sample_rate as AudioFrequencyHz,
                                                                                         &channel_encoding.block,
                                                                                         locked_callback_data.samples.as_mut_slice(),
                                                                                         need_ramp_up, need_ramp_down);
                                    debug!("Modulating {} samples", locked_callback_data.samples_written);
                                    // TODO CAT transmit enable - or pass the CAT into the thread so it
                                    // disables after it has modulated the channel_encoding.end ?
                                    // TODO How does this thread know that the modulation has ended?
                                    // It'll need a channel - or the Err block below could check the
                                    // silence flag?

                                }
                                Err(_) => {
                                    // could timeout, or be disconnected?
                                    // ignore for now...
                                    // TODO if gone silent, tell CAT to disable transmit? Need a CAT enabled flag so we don't hammer CAT to disable.
                                }
                            }
                        }
                    }
                    if need_sleep {
                        thread::sleep(Duration::from_millis(100));
                    }
                }
                debug!("Transmitter channel-encoding listener thread stopped");
            })),
            callback_data: arc_lock_modulation_callback_data,
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
        debug!("in start_callback, sample rate is {}", sample_rate);
        // Allocate the sample vector in the callback data.

        let move_clone_callback_data = self.callback_data.clone();
        let callback = move |pa::OutputStreamCallbackArgs::<f32> { buffer, frames, .. }| {
            // info!("buffer length is {}, frames is {}", buffer.len(), frames);
            // buffer length is 128, frames is 64; idx goes from [0..128).
            // One frame is a pair of left/right channel samples.
            // 48000/64=750 so in one second there are 48000 samples (frames), and 750 calls to this callback.
            // 1000/750=1.33333 so each buffer has a duration of 1.33333ms.
            //
            // An entire modulated frame has 64 symbols + 2 spare + (but there's no Costas Array yet)
            // plus possible ramp up/down ..... duration.

            let mut idx = 0;
            let mut locked_callback_data = move_clone_callback_data.write().unwrap();
            // When we start a callback and there's no data, set the silence flag true.
            if locked_callback_data.sample_index == locked_callback_data.samples_written || locked_callback_data.samples_written == 0 {
                locked_callback_data.silent.store(true, Ordering::SeqCst);
            }
            let mut is_playing = false;
            for _ in 0..frames {

                let sine_val = if locked_callback_data.sample_index < locked_callback_data.samples_written {
                    is_playing = true;
                    let this_sample = locked_callback_data.samples[locked_callback_data.sample_index];
                    locked_callback_data.sample_index += 1;
                    this_sample * locked_callback_data.amplitude_max
                } else {
                    0.0
                };

                // TODO MONO - if opening the stream with a single channel causes the same values to
                // be written to both left and right outputs, this could be optimised..
                buffer[idx] = sine_val;
                buffer[idx + 1] = sine_val;

                idx += 2;
            }
            if is_playing {
                locked_callback_data.silent.store(false, Ordering::SeqCst);
            }
            drop(locked_callback_data);
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

    // Signals the thread to terminate, blocks on joining the handle. Used by drop().
    // Setting the terminate AtomicBool will allow the thread to stop on its own, but there's no
    // method other than this for blocking until it has actually stopped.
    pub fn terminate(&mut self) {
        debug!("Terminating Transmitter");
        self.terminate.store(true, Ordering::SeqCst);
        debug!("Transmitter joining read thread handle...");
        self.thread_handle.take().map(JoinHandle::join);
        debug!("Transmitter ...joined thread handle");
    }

    // Has the thread finished (ie has it been joined)?
    pub fn terminated(&mut self) -> bool {
        debug!("Is Transmitter terminated?");
        let ret = self.thread_handle.is_none();
        debug!("Termination state is {}", ret);
        ret
    }

    pub fn set_audio_frequency_allocate_buffer(&mut self, audio_frequency: AudioFrequencyHz) -> () {
        if self.sample_rate == 0 {
            debug!("Sample rate not yet set; will set frequency when this is known");
            return;
        }
        // TODO need to pass this across to the thread, where it'll cause the phase/etc to be updated.
        {
            let mut locked_callback_data = self.callback_data.write().unwrap();
            locked_callback_data.audio_frequency = audio_frequency;
            locked_callback_data.delta_phase = 2.0_f32 * PI * (locked_callback_data.audio_frequency as f32) / (self.sample_rate as f32);
            locked_callback_data.sample_rate = self.sample_rate;
            // Calculate the maximum sample buffer size:
            let n_sym = maximum_number_of_symbols();
            debug!("maximum_number_of_symbols {}", n_sym);
            let n_spsym = (self.sample_rate as f32 * SYMBOL_PERIOD_SECONDS) as usize;
            let n_rspsym = (self.sample_rate as f32 * RAMP_SYMBOL_PERIOD_SECONDS) as usize;
            let new_sample_buffer_size = (n_sym * n_spsym) + (2 * n_rspsym); // Number of output samples, with max 2 ramping symbols
            locked_callback_data.samples = Vec::with_capacity(new_sample_buffer_size);
            locked_callback_data.samples.resize(new_sample_buffer_size, 0_f32);

            debug!("Setting transmitter frequency to {}, sample_rate {}, buffer size {}", locked_callback_data.audio_frequency, self.sample_rate, new_sample_buffer_size);
        }
    }

    pub fn set_amplitude_max(&mut self, amplitude_max: AmplitudeMax) -> () {
        if amplitude_max < 0.0 || amplitude_max > 1.0 {
            warn!("Can't set maximum amplitude outside [0.0 .. 1.0]");
            return;
        }
        let mut locked_callback_data = self.callback_data.write().unwrap();
        self.amplitude_max = amplitude_max;
        locked_callback_data.amplitude_max = amplitude_max;
    }

    pub fn is_silent(&self) -> bool {
        let locked_callback_data = self.callback_data.read().unwrap();
        locked_callback_data.silent.load(Ordering::SeqCst)
    }
}

impl BusInput<ChannelEncoding> for Transmitter {
    fn clear_input_rx(&mut self) {
        match self.input_rx.lock() {
            Ok(mut locked) => { *locked = None; }
            Err(_) => {}
        }
    }

    fn set_input_rx(&mut self, input_rx: Arc<Mutex<BusReader<ChannelEncoding>>>) {
        match self.input_rx.lock() {
            Ok(mut locked) => { *locked = Some(input_rx); }
            Err(_) => {}
        }
    }
}

impl Drop for Transmitter {
    fn drop(&mut self) {
        debug!("Transmitter signalling termination to thread on drop");
        self.terminate();
        debug!("Transmitter stopping stream...");
        self.stream.take().map(|mut r| r.stop());
        debug!("Transmitter joining thread handle...");
        self.thread_handle.take().map(JoinHandle::join);
        debug!("Transmitter ...joined thread handle");
    }
}

#[cfg(test)]
#[path = "./transmitter_spec.rs"]
mod transmitter_spec;
