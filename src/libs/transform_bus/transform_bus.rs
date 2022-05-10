use bus::{Bus, BusReader};
use log::{debug, info};
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use crate::libs::application::application::{BusInput, BusOutput};

pub struct TransformBus<I, O> where I: Clone + Sync + Send + 'static, O: Clone + Sync + Send + 'static {
    terminate_flag: Arc<AtomicBool>,
    thread_handle: Option<JoinHandle<()>>,
    // output_bus: Arc<Mutex<Bus<O>>>,
    spooky_input: PhantomData<I>,
    spooky_output: PhantomData<O>,
    input_rx: Arc<Mutex<Option<Arc<Mutex<BusReader<I>>>>>>,
    output_tx: Arc<Mutex<Option<Arc<Mutex<Bus<O>>>>>>,
}

impl<I: Clone + Sync + Send + 'static, O: Clone + Sync + Send + 'static> BusInput<I> for TransformBus<I,O> {
    fn clear_input_rx(&mut self) {
        match self.input_rx.lock() {
            Ok(mut locked) => { *locked = None; }
            Err(_) => {}
        }
    }

    fn set_input_rx(&mut self, input_rx: Arc<Mutex<BusReader<I>>>) {
        match self.input_rx.lock() {
            Ok(mut locked) => { *locked = Some(input_rx); }
            Err(_) => {}
        }
    }
}

impl<I: Clone + Sync + Send + 'static, O: Clone + Sync + Send + 'static> BusOutput<O> for TransformBus<I, O> {
    fn clear_output_tx(&mut self) {
        match self.output_tx.lock() {
            Ok(mut locked) => {
                *locked = None;
            }
            Err(_) => {}
        }
    }

    fn set_output_tx(&mut self, output_tx: Arc<Mutex<Bus<O>>>) {
        match self.output_tx.lock() {
            Ok(mut locked) => { *locked = Some(output_tx); }
            Err(_) => {}
        }
    }
}

impl<I: Clone + Sync + Send + 'static, O: Clone + Sync + Send + 'static> TransformBus<I, O> {
    pub fn new(transform: fn(I) -> O, terminate: Arc<AtomicBool>) -> Self {
        let arc_terminate = terminate.clone();
        let self_arc_terminate = arc_terminate.clone();

        // Share this holder between the TransformBus and its thread
        let input_rx_holder: Arc<Mutex<Option<Arc<Mutex<BusReader<I>>>>>> = Arc::new(Mutex::new(None));
        let thread_input_rx_holder = input_rx_holder.clone();

        // Share this holder between the TransformBus and its thread
        let output_tx_holder: Arc<Mutex<Option<Arc<Mutex<Bus<O>>>>>> = Arc::new(Mutex::new(None));
        let thread_output_tx_holder = output_tx_holder.clone();

        Self {
            terminate_flag: self_arc_terminate,
            spooky_input: PhantomData,
            spooky_output: PhantomData,
            input_rx: input_rx_holder,
            output_tx: output_tx_holder,
            thread_handle: Some(thread::spawn(move || {
                info!("TransformBus thread started");
                loop {
                    if arc_terminate.load(Ordering::SeqCst) {
                        info!("Terminating TransformBus thread");
                        break;
                    }

                    match thread_input_rx_holder.lock().unwrap().as_ref() {
                        None => {
                            // Input channel hasn't been set yet
                            thread::sleep(Duration::from_millis(100));
                        }
                        Some(input_rx) => {
                            match input_rx.lock().unwrap().recv_timeout(Duration::from_millis(250)) {
                                Ok(input) => {
                                    match thread_output_tx_holder.lock().unwrap().as_ref() {
                                        None => {
                                            // Output channel hasn't been set yet
                                            thread::sleep(Duration::from_millis(100));
                                        }
                                        Some(output_tx) => {
                                            output_tx.lock().unwrap().broadcast(transform(input));
                                        }
                                    }
                                }
                                Err(_) => {
                                    // could timeout, or be disconnected?
                                    // ignore for now...
                                }
                            }
                        }
                    }
                }
                debug!("TransformBus thread stopped");
            })),
        }
    }

    // Signals the thread to terminate, blocks on joining the handle. Used by drop().
    // Setting the terminate AtomicBool will allow the thread to stop on its own, but there's no
    // method other than this for blocking until it has actually stopped.
    pub fn terminate(&mut self) {
        debug!("Terminating TransformBus");
        self.terminate_flag.store(true, core::sync::atomic::Ordering::SeqCst);
        debug!("TransformBus joining read thread handle...");
        self.thread_handle.take().map(JoinHandle::join);
        debug!("TransformBus ...joined thread handle");
    }

    // Has the thread finished (ie has it been joined)?
    pub fn terminated(&mut self) -> bool {
        debug!("Is TransformBus terminated?");
        let ret = self.thread_handle.is_none();
        debug!("Termination state is {}", ret);
        ret
    }

    pub fn add_reader(&mut self) -> BusReader<O> {
        loop {
            match self.output_tx.lock().unwrap().as_ref() {
                None => {
                    // Output channel hasn't been set yet
                    thread::sleep(Duration::from_millis(100));
                }
                Some(output_tx) => {
                    return output_tx.lock().unwrap().add_rx();
                }
            }
        }
    }
}

impl<I: Clone + Sync + Send + 'static, O: Clone + Sync + Send + 'static> Drop for TransformBus<I, O> {
    fn drop(&mut self) {
        debug!("TransformBus signalling termination to thread on drop");
        self.terminate();
    }
}



#[cfg(test)]
#[path = "./transform_bus_spec.rs"]
mod transform_bus_spec;
