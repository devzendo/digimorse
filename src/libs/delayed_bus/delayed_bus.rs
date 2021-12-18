use log::{debug, info};
use std::fmt::Display;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use bus::{Bus, BusReader};
use syncbox::{ScheduledThreadPool, Task};

pub struct DelayedBus<T> where T: 'static + Send + Display + Clone + Sync {
    terminate_flag: Arc<AtomicBool>,
    read_thread_handle: Mutex<Option<JoinHandle<()>>>,
    spooky: PhantomData<T>,
}

impl<T: 'static + Send + Display + Clone + Sync> DelayedBus<T> {
    pub fn new(input_rx: BusReader<T>, output_tx: Bus<T>, terminate: Arc<AtomicBool>, delay: Duration) -> Self {
        let arc_terminate = terminate.clone();
        let arc_output_tx = Arc::new(Mutex::new(output_tx));
        let read_thread_handle = thread::spawn(move || {
            let mut read_thread = DelayedBusReadThread::new(delay, input_rx,
                                                            arc_output_tx,
                                                            arc_terminate);
            read_thread.thread_runner();
        });

        Self {
            terminate_flag: terminate,
            read_thread_handle: Mutex::new(Some(read_thread_handle)),
            spooky: PhantomData,
        }
    }

    // Signals the thread to terminate, blocks on joining the handle. Used by drop().
    // Setting the terminate AtomicBool will allow the thread to stop on its own, but there's no
    // method other than this for blocking until it has actually stopped.
    pub fn terminate(&mut self) {
        debug!("Terminating delayed bus");
        self.terminate_flag.store(true, core::sync::atomic::Ordering::SeqCst);
        debug!("DelayedBus joining read thread handle...");
        let mut read_thread_handle = self.read_thread_handle.lock().unwrap();
        read_thread_handle.take().map(JoinHandle::join);
        debug!("DelayedBus ...joined thread handle");
    }

    // Has the thread finished (ie has it been joined)?
    pub fn terminated(&mut self) -> bool {
        debug!("Is delayed bus terminated?");
        let ret = self.read_thread_handle.lock().unwrap().is_none();
        debug!("Termination state is {}", ret);
        ret
    }

    // fn emit(&mut self) {
    //     self.shared.lock().unwrap().emit();
    // }
}

impl<T: Send + Display + Clone + Sync + 'static> Drop for DelayedBus<T> {
    fn drop(&mut self) {
        debug!("DelayedBus signalling termination to thread on drop");
        self.terminate();
    }
}


struct DelayedBusReadThread<T> {
    delay: Duration,
    terminate: Arc<AtomicBool>,
    input_rx: BusReader<T>,
    output_tx: Arc<Mutex<Bus<T>>>,
    scheduled_thread_pool: ScheduledThreadPool,
}


impl<T: Send + Display + Clone + Sync + 'static> DelayedBusReadThread<T> {
    fn new(delay: Duration,
           input_rx: BusReader<T>,
           output_tx: Arc<Mutex<Bus<T>>>,
           terminate: Arc<AtomicBool>,
    ) -> Self {
        debug!("Constructing DelayedBusReadThread");
        let scheduled_thread_pool = syncbox::ScheduledThreadPool::single_thread();

        Self {
            delay,
            terminate,
            scheduled_thread_pool,
            input_rx,
            output_tx,
        }
    }

    // Thread that handles incoming T and delays them
    fn thread_runner(&mut self) -> () {
        info!("DelayedBus thread started");
        loop {
            if self.terminate.load(core::sync::atomic::Ordering::SeqCst) {
                info!("Terminating DelayedBus");
                break;
            }

            match self.input_rx.recv_timeout(Duration::from_millis(100)) {
                Ok(item) => {
                    debug!("Received item {}", item);
                    let item_later = self.delay.as_millis();
                    let arc_output_tx = self.output_tx.clone();
                    let task = TimedOutput{ item, output_tx: arc_output_tx };
                    self.scheduled_thread_pool.schedule_ms(item_later as u32, task);
                }
                Err(_) => {
                    // Don't log, it's just noise - timeout gives opportunity to go round loop and
                    // check for terminate.
                }
            }
        }
        info!("DelayedBus thread stopped");
    }
}

struct TimedOutput<T> {
    item: T,
    output_tx: Arc<Mutex<Bus<T>>>,
}

impl<T: Display + Send + Clone + Sync + 'static> Task for TimedOutput<T> {
    fn run(self) {
        debug!("Broadcasting item {}", self.item);
        let mut output = self.output_tx.lock().unwrap();
        output.broadcast(self.item);
    }
}

#[cfg(test)]
#[path = "./delayed_bus_spec.rs"]
mod delayed_bus_spec;
