use std::error::Error;
use std::sync::{Arc, Mutex, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use bus::Bus;
use log::{debug, info, warn};
use portaudio::{NonBlocking, Input, InputStreamSettings, PortAudio, Stream};
use portaudio as pa;
use crate::libs::application::application::BusOutput;
use crate::libs::buffer_pool::observable_buffer::{ObservableBuffer, ObservableBufferSlice};
use crate::libs::patterns::observer::Observer;
use crate::libs::transmitter::transmitter::{AmplitudeMax, AudioFrequencyHz};

pub struct Receiver {
    audio_offset: AudioFrequencyHz,
    amplitude_max: AmplitudeMax,
    sample_rate: u32,
    stream: Option<Stream<NonBlocking, Input<f32>>>,
    callback_data: Arc<RwLock<CallbackData>>,
    terminate: Arc<AtomicBool>,
    observable_buffer: ObservableBuffer<f32>,
}

#[derive(Clone, PartialEq, Copy)]
pub enum ReceiverEvent {
    // TODO FFT data    
}

struct CallbackData {
    injected_waveform: Option<InjectedWaveform>,
    output_tx: Option<Arc<Mutex<Bus<ReceiverEvent>>>>,

}

struct InjectedWaveform {
    waveform: Vec<f32>,
    playback_index: usize,
}

impl Receiver {

    pub fn new(audio_offset: AudioFrequencyHz, terminate: Arc<AtomicBool>,
               /* TODO CAT controller passed in here */) -> Self {
        let callback_data = CallbackData { injected_waveform: None, output_tx: None };
        Self {
            audio_offset: audio_offset,
            amplitude_max: 1.0,
            sample_rate: 0,
            stream: None,
            callback_data: Arc::new(RwLock::new(callback_data)),
            terminate,
            observable_buffer: ObservableBuffer::new(),
        }
    }

    pub fn add_observer(&mut self, observer: Arc<dyn Observer<ObservableBufferSlice<f32>>>) {
        self.observable_buffer.add_observer(observer);
    }

    pub fn set_amplitude_max(&mut self, amplitude_max: AmplitudeMax) {
        if amplitude_max < 0.0 || amplitude_max > 1.0 {
            warn!("Can't set maximum amplitude outside [0.0 .. 1.0]");
            return;
        }
        debug!("Setting receiver amplitude to {}", amplitude_max);
        self.amplitude_max = amplitude_max;
        // let mut locked_callback_data = self.callback_data.write().unwrap();
        // locked_callback_data.amplitude_max = amplitude_max;
    }

    // The odd form of this callback setup (pass in the PortAudio and settings) rather than just
    // returning the callback to the caller to do stuff with... is because I can't work out what
    // the correct type signature of a callback-returning function should be.
    pub fn start_callback(&mut self, pa: &PortAudio, mut input_settings: InputStreamSettings<f32>) -> Result<(), Box<dyn Error>> {
        let sample_rate = input_settings.sample_rate as u32;
        self.sample_rate = sample_rate;
        debug!("in start_callback, sample rate is {}", sample_rate);

        let callback = move |pa::InputStreamCallbackArgs::<f32> { buffer, frames, .. }| {
            // info!("buffer length is {}, frames is {}", buffer.len(), frames);
            // buffer length is 64, frames is 64

            // TODO is there a waveform to inject?
            // Downsampled audio is collected into a circular buffer. Every 40ms, the last 160ms
            // of audio is emitted to observers (after we've received the first 160ms, of course).
            // The FFT observer zero-pads this 160ms audio to 320ms, and transforms, and emits that
            // to its observers.
            // The input rate is 48000Hz. Each ms there are 48 samples. We're downsampling by 4, so
            // each ms has 12 downsamples. 160ms therefore contains 12 samples * 160 ms = 1920 samples.
            // The circular buffer needs to hold twice as much as this to prevent collisions.
            pa::Continue
        };

        let maybe_stream = pa.open_non_blocking_stream(input_settings, callback);
        match maybe_stream {
            Ok(mut stream) => {
                stream.start()?;
                self.stream = Some(stream);
            }
            Err(e) => {
                warn!("Error opening receiver input stream: {}", e);
            }
        }
        Ok(())
        // Now it's listening...
    }


    // Signals the thread to terminate, blocks on joining the handle. Used by drop().
    // Setting the terminate AtomicBool will allow the thread to stop on its own, but there's no
    // method other than this for blocking until it has actually stopped.
    pub fn terminate(&mut self) {
        debug!("Terminating Receiver");
        self.terminate.store(true, Ordering::SeqCst);
        // debug!("Receiver joining read thread handle...");
        // self.thread_handle.take().map(JoinHandle::join);
        // debug!("Receiver ...joined thread handle");
    }

    // Has the thread finished (ie has it been joined)?
    // pub fn terminated(&mut self) -> bool {
    //     debug!("Is Receiver terminated?");
    //     let ret = self.thread_handle.is_none();
    //     debug!("Termination state is {}", ret);
    //     ret
    // }

    pub fn set_audio_frequency(&mut self, audio_frequency: AudioFrequencyHz) {
        if self.sample_rate == 0 {
            debug!("Sample rate not yet set; will set frequency when this is known");
            return;
        }
        self.audio_offset = audio_frequency;
        // TODO need to pass this across to the thread, where it'll cause the phase/etc to be updated.
        {
            // let mut locked_callback_data = self.callback_data.write().unwrap();
            // locked_callback_data.audio_frequency = audio_frequency;
            // locked_callback_data.delta_phase = 2.0_f32 * PI * (locked_callback_data.audio_frequency as f32) / (self.sample_rate as f32);
            // locked_callback_data.sample_rate = self.sample_rate;
            // debug!("Setting transmitter frequency to {}, sample_rate {}, buffer size {}", locked_callback_data.audio_frequency, self.sample_rate, new_sample_buffer_size);
        }
    }

    pub fn inject_waveform(&mut self, waveform: &Vec<f32>) -> () {
        let mut locked_callback_data = self.callback_data.write().unwrap();
        locked_callback_data.injected_waveform = Some(InjectedWaveform { waveform: waveform.clone(), playback_index: 0 });
        info!("Injecting waveform of {} samples", waveform.len());
    }
}

impl BusOutput<ReceiverEvent> for Receiver {
    fn clear_output_tx(&mut self) {
        let _locked_callback_data = self.callback_data.write().unwrap();
        // TODO TDD locked_callback_data.output_tx = None;
    }

    fn set_output_tx(&mut self, _output_tx: Arc<Mutex<Bus<ReceiverEvent>>>) {
        let _locked_callback_data = self.callback_data.write().unwrap();
        // TODO TDD locked_callback_data.output_tx = Some(output_tx);
    }
}


impl Drop for Receiver {
    fn drop(&mut self) {
        debug!("Receiver signalling termination to thread on drop");
        self.terminate();
        debug!("Receiver stopping stream...");
        self.stream.take().map(|mut r| r.stop());
        // debug!("Receiver joining thread handle...");
        // self.thread_handle.take().map(JoinHandle::join);
        // debug!("Receiver ...joined thread handle");
    }
}

#[cfg(test)]
#[path = "./receiver_spec.rs"]
mod receiver_spec;
