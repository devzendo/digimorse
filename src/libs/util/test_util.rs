use std::sync::mpsc;
use std::thread;
use std::time::Duration;

// Thanks to Shepmaster, https://github.com/rust-lang/rfcs/issues/2798
pub fn panic_after<T, F>(d: Duration, f: F) -> T
    where
        T: Send + 'static,
        F: FnOnce() -> T,
        F: Send + 'static,
{
    let (done_tx, done_rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        let val = f();
        done_tx.send(()).expect("Unable to send completion signal");
        val
    });

    match done_rx.recv_timeout(d) {
        Ok(_) => handle.join().expect("Thread panicked"),
        Err(_) => panic!("Thread took too long"),
    }
}

pub fn wait_5_ms() {
    wait_n_ms(5);
}

pub fn wait_n_ms(n: u64) {
    thread::sleep(Duration::from_millis(n));
}

