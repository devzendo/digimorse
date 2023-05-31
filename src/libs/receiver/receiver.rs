use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use log::{debug, info, warn};
use portaudio::{NonBlocking, Input, InputStreamSettings, PortAudio, Stream};
use portaudio as pa;
use crate::libs::transmitter::transmitter::{AmplitudeMax, AudioFrequencyHz};

pub struct Receiver {
    audio_offset: AudioFrequencyHz,
    amplitude_max: AmplitudeMax,
    sample_rate: u32,
    stream: Option<Stream<NonBlocking, Input<f32>>>,
    terminate: Arc<AtomicBool>,

}

impl Receiver {

    pub fn new(audio_offset: AudioFrequencyHz, terminate: Arc<AtomicBool>,
               /* TODO CAT controller passed in here */) -> Self {
        Self {
            audio_offset: audio_offset,
            amplitude_max: 1.0,
            sample_rate: 0,
            stream: None,
            terminate,
        }
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
