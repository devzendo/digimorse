use log::{debug, info};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};
use bus::{Bus, BusReader};

#[readonly::make]
pub struct DelayedBus<T> {
    terminate: Arc<AtomicBool>,
    read_thread_handle: Mutex<Option<JoinHandle<()>>>,
    shared: Arc<Mutex<DelayedBusShared<T>>>,
}

impl<T: Send> DelayedBus<T> {
    pub fn new(input_rx: BusReader<T>, output_tx: Bus<T>, terminate: Arc<AtomicBool>, delay: Duration) -> Self {
        let arc_terminate = terminate.clone();
        let mut heap = BinaryHeap::new();

        let shared = Mutex::new(DelayedBusShared {
            heap,
            output_tx,
        });
        let arc_shared = Arc::new(shared);
        let arc_shared_cloned = arc_shared.clone();
        let read_thread_handle = thread::spawn(move || {
            let mut read_thread = DelayedBusReadThread::new(delay, input_rx,
                                                            arc_terminate,
                                                            arc_shared.clone());
            read_thread.thread_runner();
        });

        Self {
            terminate,
            read_thread_handle: Mutex::new(Some(read_thread_handle)),
            shared: arc_shared_cloned,
        }
    }

    // Signals the thread to terminate, blocks on joining the handle. Used by drop().
    // Setting the terminate AtomicBool will allow the thread to stop on its own, but there's no
    // method other than this for blocking until it has actually stopped.
    pub fn terminate(&mut self) {
        debug!("Terminating delayed bus");
        self.terminate.store(true, core::sync::atomic::Ordering::SeqCst);
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

impl<T> Drop for DelayedBus<T> {
    fn drop(&mut self) {
        debug!("DelayedBus signalling termination to thread on drop");
        self.terminate();
    }
}


// An object shared between the main DelayedBus, and the DelayedBusThread.
struct DelayedBusShared<T> {
    output_tx: Bus<T>,
    heap: BinaryHeap<TimedT<T>>,
}

impl<T: Debug> DelayedBusShared<T> {
    fn emit(&mut self, item: T) {
        debug!("Emitting {:?}", item);
        self.output_tx.broadcast(item);
    }
}


struct DelayedBusReadThread<T> {
    delay: Duration,
    terminate: Arc<AtomicBool>,
    input_rx: BusReader<T>,

    // Shared state between thread and main code
    shared: Arc<Mutex<DelayedBusShared<T>>>,
}

#[derive(Copy, Clone)]
struct TimedT<T> {
    millis_since_epoch: u128,
    item: T,
}

impl<T> Eq for TimedT<T> {}

impl<T> PartialEq<Self> for TimedT<T> {
    fn eq(&self, other: &Self) -> bool {
        self.millis_since_epoch == other.millis_since_epoch
    }
}

impl<T> PartialOrd<Self> for TimedT<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// The priority queue depends on `Ord`.
// Explicitly implement the trait so the queue becomes a min-heap
// instead of a max-heap.
impl<T> Ord for TimedT<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        other.millis_since_epoch.cmp(&self.millis_since_epoch)
    }
}

impl<T> DelayedBusReadThread<T> {
    fn new(delay: Duration,
           input_rx: BusReader<T>,
           terminate: Arc<AtomicBool>,
           shared: Arc<Mutex<DelayedBusShared<T>>>
    ) -> Self {
        debug!("Constructing DelayedBusReadThread");

        Self {
            delay,
            terminate,
            input_rx,
            shared,
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
                    let item_millis_since_epoch = get_epoch_ms();
                    let item_later = self.delay.as_millis() + item_millis_since_epoch;
                    let to_queue = TimedT::<T> { millis_since_epoch: item_later, item };
                    self.shared.lock().unwrap().heap.push(to_queue);
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

fn get_epoch_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}


#[cfg(test)]
#[path = "./delayed_bus_spec.rs"]
mod delayed_bus_spec;
