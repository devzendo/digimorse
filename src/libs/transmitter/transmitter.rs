use std::collections::VecDeque;
use std::error::Error;
use std::f32::consts::PI;
use std::sync::{Arc, Mutex, RwLock, RwLockWriteGuard};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use bus::BusReader;
use fp_rust::sync::CountDownLatch;
use log::{debug, error, info, warn};
use portaudio::{NonBlocking, Output, OutputStreamSettings, PortAudio, Stream};
use portaudio as pa;
use crate::libs::application::application::BusInput;
use crate::libs::buffer_pool::buffer_pool::BufferPool;
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
    silent: Arc<AtomicBool>,

    // Shared between thread and Transmitter
    input_rx: Arc<Mutex<Option<Arc<Mutex<BusReader<ChannelEncoding>>>>>>,
}

struct CallbackData {
    _amplitude: f32, // used for ramping up/down output waveform at start and end
    audio_frequency: AudioFrequencyHz,
    amplitude_max: AmplitudeMax,
    delta_phase: f32, // added to the phase after recording each sample
    _phase: f32,       // sin(phase) is the sample value
    sample_rate: u32, // Hz
    samples: Vec<f32>, // contains the GFSK modulated waveform to emit, allocated as a Vec, used as a slice
    buffer_pool: Arc<Mutex<Option<BufferPool>>>, // allocated when sample rate known
    callback_messages: VecDeque<CallbackMessage>, // buffers to emit, or latches to sync on
}

const NUMBER_OF_BUFFERS: usize = 32;

struct BufferIndex {
    index: usize,
    buffer: Arc<RwLock<Vec<f32>>>,
    buffer_index: usize,
    buffer_max: usize,
}

enum CallbackMessage {
    BufferIndex(BufferIndex),
    Wait(Arc<CountDownLatch>),
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
        let no_buffer_pool = Arc::new(Mutex::new(None));
        let modulation_callback_data = CallbackData {
            _amplitude: 0.0,
            audio_frequency: audio_offset,
            amplitude_max: 1.0,
            delta_phase: 0.0,
            _phase: 0.0,
            sample_rate: 0,
            samples: vec![],
            buffer_pool: no_buffer_pool,
            callback_messages: VecDeque::new(),
        };
        // TODO replace this Mutex with atomics to reduce contention in the callback.
        let arc_lock_modulation_callback_data = Arc::new(RwLock::new(modulation_callback_data));
        let move_clone_modulation_callback_data = arc_lock_modulation_callback_data.clone();
        let move_clone_modulation_silent = silent.clone();


        Self {
            _radio_frequency_mhz: 0,
            _audio_offset: 0,
            amplitude_max: 1.0,
            sample_rate: 0, // will be initialised when the callback is initialised
            dt: 0.0,        // will be initialised when the callback is initialised
            terminate: terminate.clone(),
            input_rx: input_rx_holder,    // Modified by BusInput
            silent: silent.clone(),
            thread_handle: Some(thread::spawn(move || {
                info!("Transmitter channel-encoding listener thread started");
                loop {
                    if terminate.load(Ordering::SeqCst) {
                        info!("Terminating transmitter thread");
                        break;
                    }

                    // If silent when a channel encoding arrives, this indicates that we are
                    // starting a transmission, and that we should PTT via CAT, and use a ramp up
                    // symbol at the start of the modulation.
                    // This channel-reading loop is where the CAT work needs doing; the callback
                    // should only be concerned with returning the samples to PortAudio.
                    // However this part of the code is where we know when we've reached the end of
                    // transmission, since the is_end field of the channel encoding indicates this.
                    // So then, we un-PTT via CAT. However, we need to sense when the callback has
                    // finished transmitting the last block (the is_end one).
                    // Communication between this message-reading thread and the callback is done
                    // by allocating a buffer from the buffer pool, modulating the channel encoding
                    // into it, and passing the index of the buffer to the callback via its work
                    // queue vector. If this is the end buffer, also append a CountDownLatch to the
                    // work queue vector, which the callback will count down when it has finished
                    // modulating. This thread will be awaiting it; it will then disable CAT.
                    //
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
                                    let mut maybe_countdown_latch: Option<Arc<CountDownLatch>> = None;
                                    let mut locked_callback_data = move_clone_modulation_callback_data.write().unwrap();
                                    let need_ramp_up = move_clone_modulation_silent.load(Ordering::SeqCst);
                                    let need_ramp_down = channel_encoding.is_end;
                                    info!("Ramp up {} down {}", need_ramp_up, need_ramp_down);
                                    if need_ramp_up {
                                        // TODO CAT transmit enable.
                                        // watch out - locked_callback_data lock is held
                                    }
                                    let maybe_allocated_modulated_buffer: Option<(usize, Arc<RwLock<Vec<f32>>>, usize)> = allocate_buffer_and_write_modulation(&locked_callback_data, &channel_encoding, need_ramp_up, need_ramp_down, locked_callback_data.audio_frequency, locked_callback_data.sample_rate as AudioFrequencyHz);
                                    match maybe_allocated_modulated_buffer {
                                        None => {}
                                        Some((index, buffer, buffer_max)) => {
                                            info!("Enqueueing buffer {} with {} samples: queue has {} items", index, buffer_max, locked_callback_data.callback_messages.len());
                                            locked_callback_data.callback_messages.push_back(
                                                CallbackMessage::BufferIndex(BufferIndex { index, buffer, buffer_index: 0, buffer_max} ));
                                        }
                                    }
                                    if need_ramp_down {
                                        info!("Enqueueing countdown latch: queue has {} items", locked_callback_data.callback_messages.len());
                                        let countdown_latch = Arc::new(CountDownLatch::new(1));
                                        maybe_countdown_latch = Some(countdown_latch.clone());
                                        locked_callback_data.callback_messages.push_back(
                                            CallbackMessage::Wait(countdown_latch));
                                    }
                                    drop(locked_callback_data);

                                    // If this was an end buffer, wait for modulation to finish via the synchronising
                                    // CountDownLatch.
                                    if let Some(latch) = maybe_countdown_latch {
                                        info!("Waiting for end of modulation");
                                        latch.wait();
                                        info!("End of modulation signalled");
                                        // TODO CAT transmit disable
                                    }
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
                info!("Transmitter channel-encoding listener thread stopped");
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
        let move_clone_callback_silent = self.silent.clone();
        let callback = move |pa::OutputStreamCallbackArgs::<f32> { buffer, frames, .. }| {

            let set_silent = |silent: bool| {
                if silent != move_clone_callback_silent.swap(silent, Ordering::SeqCst) {
                    info!("Changed silent flag to {}", silent);
                }
            };

            // info!("buffer length is {}, frames is {}", buffer.len(), frames);
            // buffer length is 128, frames is 64; idx goes from [0..128).
            // One frame is a pair of left/right channel samples.
            // 48000/64=750 so in one second there are 48000 samples (frames), and 750 calls to this callback.
            // 1000/750=1.33333 so each buffer has a duration of 1.33333ms.
            //
            // An entire modulated frame has 64 symbols + 2 spare + (but there's no Costas Array yet)
            // plus possible ramp up/down ..... duration.

            let mut locked_callback_data = move_clone_callback_data.write().unwrap();
            let amplitude_max = locked_callback_data.amplitude_max;
            // When we start a callback and there's no data, set the silence flag true.
            if locked_callback_data.callback_messages.is_empty() {
                debug!("Silence: true (no callback_messages)");
                set_silent(true);
                let mut idx = 0;
                for _ in 0..frames {
                    buffer[idx] = 0.0;
                    buffer[idx + 1] = 0.0;
                    idx += 2;
                }
            } else {
                debug!("Silence: false (some callback_messages)");
                set_silent(false);
                let first = locked_callback_data.callback_messages.front_mut().unwrap();
                let mut maybe_buffer_free_index: Option<usize> = None;
                match first {
                    CallbackMessage::BufferIndex(bi) => {
                        debug!("Sample index at callback start: {}, samples written {}", bi.buffer_index, bi.buffer_max);
                        let mut idx = 0;
                        let locked_samples = bi.buffer.read().unwrap();
                        for _ in 0..frames {
                            let sine_val = if bi.buffer_index < bi.buffer_max {
                                let this_sample = locked_samples[bi.buffer_index];
                                bi.buffer_index += 1;
                                this_sample * amplitude_max
                            } else {
                                0.0
                            };

                            // TODO MONO - if opening the stream with a single channel causes the same values to
                            // be written to both left and right outputs, this could be optimised..
                            buffer[idx] = sine_val;
                            buffer[idx + 1] = sine_val;
                            idx += 2;
                        }
                        drop(locked_samples);
                        if bi.buffer_index == bi.buffer_max {
                            // Free the index outside the current borrow of locked_callback_data...
                            debug!("Want to free buffer {}", bi.index);
                            maybe_buffer_free_index = Some(bi.index);
                            locked_callback_data.callback_messages.pop_front();
                        }
                    }
                    CallbackMessage::Wait(arc_latch) => {
                        info!("Notifying end of modulation");
                        arc_latch.countdown();
                        info!("Notified end of modulation");
                        locked_callback_data.callback_messages.pop_front();
                    }
                }
                if let Some(to_free_index) = maybe_buffer_free_index {
                    free_buffer(&locked_callback_data, to_free_index);
                }
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

    pub fn set_audio_frequency_allocate_buffer(&mut self, audio_frequency: AudioFrequencyHz) {
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
            if let Ok(mut locked) = locked_callback_data.buffer_pool.lock() {
                *locked = Some(BufferPool::new(new_sample_buffer_size, NUMBER_OF_BUFFERS))
            }

            debug!("Setting transmitter frequency to {}, sample_rate {}, buffer size {}", locked_callback_data.audio_frequency, self.sample_rate, new_sample_buffer_size);
        }
    }

    pub fn set_amplitude_max(&mut self, amplitude_max: AmplitudeMax) {
        if amplitude_max < 0.0 || amplitude_max > 1.0 {
            warn!("Can't set maximum amplitude outside [0.0 .. 1.0]");
            return;
        }
        debug!("Setting transmitter amplitude to {}", amplitude_max);
        let mut locked_callback_data = self.callback_data.write().unwrap();
        self.amplitude_max = amplitude_max;
        locked_callback_data.amplitude_max = amplitude_max;
    }

    pub fn is_silent(&self) -> bool {
        self.silent.load(Ordering::SeqCst)
    }
}

fn free_buffer(locked_callback_data: &RwLockWriteGuard<CallbackData>, to_free_index: usize) {
    match locked_callback_data.buffer_pool.lock().unwrap().as_mut() {
        None => {
            error!("Want to free but no buffer pool present when channel encodings received");
        }
        Some(buffer_pool) => {
            debug!("Freeing buffer {}", to_free_index);
            buffer_pool.free(to_free_index);
        }
    }
}

fn allocate_buffer_and_write_modulation(locked_callback_data: &RwLockWriteGuard<CallbackData>, channel_encoding: &ChannelEncoding, need_ramp_up: bool, need_ramp_down: bool, offset_frequency: AudioFrequencyHz, sample_rate: AudioFrequencyHz) -> Option<(usize, Arc<RwLock<Vec<f32>>>, usize)> {
    match locked_callback_data.buffer_pool.lock().unwrap().as_mut() {
        None => {
            error!("No buffer pool present when channel encodings received");
            None
        }
        Some(buffer_pool) => {
            // Obtain a buffer from the pool, convert the
            // channel_encoding into a GFSK waveform, and store it in
            // the buffer, then add it to the callback_messages for the
            // callback to emit.
            match buffer_pool.allocate() {
                Some((index, arc_samples)) => {
                    let mut locked_samples = arc_samples.write().unwrap();
                    debug!("waveform store has {} space", locked_samples.capacity());
                    let arc_samples_slice = locked_samples.as_mut_slice();
                    let samples_written = gfsk_modulate(
                        offset_frequency,
                        sample_rate,
                        &channel_encoding.block,
                        arc_samples_slice,
                        need_ramp_up, need_ramp_down);
                    drop(locked_samples);
                    Some((index, arc_samples, samples_written))
                }
                None => {
                    error!("Cannot modulate channel encoding");
                    None
                },
            }
        }
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
