// Allow using Mutex<bool> to support Mutex/Condvar pattern
#![allow(clippy::mutex_atomic)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

/// Represents a timer that can be iterated on and will block until either
/// stopped or signalled by its worker thread to emit another tick. The specific
/// implementation ensures that the timer ticks will be as close to the target
/// interval as possible (using `std::sync::mpsc::Receiver::recv_timeout` as the
/// timing mechanism) due to a separate thread doing the waiting. This means
/// that this timer thread can signal and then immediately wait for the next
/// interval without being slowed by the processing time for the previous tick.
pub struct Timer {
    pub duration: Duration,
    shared:       Arc<SharedTimerState>,
}

/// Represents a cloneable handle to stop a timer running its own worker thread
pub struct Stopper {
    shared: Arc<SharedTimerState>,
}

/// Shared concurrency control data structures used to synchronize a timer
struct SharedTimerState {
    stopping:    AtomicBool,
    lock:        Mutex<bool>,
    signal_tick: Condvar,
    tx_stop:     Mutex<Sender<()>>,
}

/// Represents a timer or timer-like object that can be stopped
pub trait Stoppable {
    fn stop(&self);
}

impl Timer {
    #[must_use]
    pub fn new(dur: Duration) -> (Self, Stopper) {
        let (tx_stop, rx_stop): (Sender<()>, Receiver<()>) = mpsc::channel();
        let shared = Arc::new(SharedTimerState {
            stopping:    AtomicBool::new(false),
            lock:        Mutex::new(false),
            signal_tick: Condvar::new(),
            tx_stop:     Mutex::new(tx_stop),
        });

        // Spawn the timer thread
        let shared_c = Arc::clone(&shared);
        thread::spawn(move || {
            loop {
                // Signal the receiving thread to wake up and perform the timer
                // action (without stopping)
                let mut signal = shared_c.lock.lock().unwrap();
                *signal = true;
                shared_c.signal_tick.notify_one();
                // Drop the mutex to prevent deadlock
                drop(signal);

                // Use recv_timeout as the sleep mechanism to allow for early
                // waking
                let recv_result = rx_stop.recv_timeout(dur);
                if recv_result.is_ok() {
                    // An empty message was sent on rx_stop, so stop the timer
                    // immediately
                    break;
                }
            }
        });

        let shared_c = Arc::clone(&shared);
        (
            Self {
                duration: dur,
                shared,
            },
            Stopper { shared: shared_c },
        )
    }
}

/// Performs the internal logic to stop and then signal an update to the
/// listening thread
fn stop_timer(shared: &SharedTimerState) {
    shared.stopping.store(true, Ordering::SeqCst);
    let mut signal = shared.lock.lock().unwrap();
    *signal = true;
    drop(signal);

    let tx_stop = shared.tx_stop.lock().unwrap();
    // ignore result: if the channel was closed, then the receiver (timer
    // thread) must already have exited
    let _ = tx_stop.send(());
    drop(tx_stop);

    shared.signal_tick.notify_one();
}

impl Stoppable for Timer {
    /// Stops the timer thread when it checks on the next tick and immediately
    /// stops iteration
    fn stop(&self) { stop_timer(&self.shared); }
}

impl Stoppable for Stopper {
    /// Stops the timer thread and the thread blocked on the iteration
    /// immediately
    fn stop(&self) { stop_timer(&self.shared); }
}

impl Drop for Timer {
    /// Stops the timer thread and the thread blocked on the iteration
    /// immediately
    fn drop(&mut self) {
        // If the stopping mechanisms have already been triggered, then skip
        if !self.shared.stopping.load(Ordering::SeqCst) {
            self.stop();
        }
    }
}

impl Iterator for Timer {
    type Item = ();

    /// Blocks the current thread until the next timer action, or returns None
    /// if the timer has stopped. Called by the listening thread
    fn next(&mut self) -> Option<Self::Item> {
        let mut next_tick = self.shared.lock.lock().unwrap();
        while !*next_tick {
            next_tick = self.shared.signal_tick.wait(next_tick).unwrap();
        }
        *next_tick = false;

        // If stopping was flagged, then stop. Else, return an empty option to
        // yield to the caller and let them process the next tick
        if self.shared.stopping.load(Ordering::SeqCst) {
            None
        } else {
            Some(())
        }
    }
}

impl Clone for Stopper {
    fn clone(&self) -> Self {
        Self {
            shared: Arc::clone(&self.shared),
        }
    }
}
