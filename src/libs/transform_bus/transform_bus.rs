use bus::{Bus, BusReader};
use log::{debug, info};
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

pub struct TransformBus<I, O> where I: Clone + Sync + Send + 'static, O: Clone + Sync + Send + 'static {
    terminate_flag: Arc<AtomicBool>,
    thread_handle: Option<JoinHandle<()>>,
    output_bus: Arc<Mutex<Bus<O>>>,
    spooky_input: PhantomData<I>,
    spooky_output: PhantomData<O>,
}

impl<I: Clone + Sync + Send + 'static, O: Clone + Sync + Send + 'static> TransformBus<I, O> {
    pub fn new(mut input: BusReader<I>, output: Arc<Mutex<Bus<O>>>, transform: fn(I) -> O, terminate: Arc<AtomicBool>) -> Self {
        let arc_terminate = terminate.clone();
        let self_arc_terminate = arc_terminate.clone();
        let self_arc_output_bus = output.clone();
        let arc_output_bus = output.clone();
        Self {
            terminate_flag: self_arc_terminate,
            output_bus: self_arc_output_bus,
            spooky_input: PhantomData,
            spooky_output: PhantomData,
            thread_handle: Some(thread::spawn(move || {
                info!("TransformBus thread started");
                loop {
                    if arc_terminate.load(Ordering::SeqCst) {
                        info!("Terminating TransformBus thread");
                        break;
                    }

                    match input.recv_timeout(Duration::from_millis(250)) {
                        Ok(input) => {
                            arc_output_bus.lock().unwrap().broadcast(transform(input));
                        }
                        Err(_) => {
                            // could timeout, or be disconnected?
                            // ignore for now...
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
        return self.output_bus.lock().unwrap().add_rx();
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
