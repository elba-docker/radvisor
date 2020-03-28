use std::marker::Send;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Represents a timer that can be iterated on and will block until either
/// stopped or signalled by its worker thread to emit another tick
pub struct Timer {
    pub interval: Duration,
    shared:       Arc<SharedTimerState>,
}

unsafe impl Send for Timer {}

/// Represents a cloneable handle to stop a timer running its own worker thread
pub struct TimerStopper {
    shared: Arc<SharedTimerState>,
}

/// Shared concurrency control data structures used to synchronize a timer
struct SharedTimerState {
    /// Used to send stop signals to immediately interrupt the sleeping thread
    tx_stop: Mutex<Sender<()>>,
    /// Used to receive stop signals (**will be blocking for long periods of
    /// time**)
    rx_stop: Mutex<Receiver<()>>,
}

/// Represents a timer or timer-like object that can be stopped
pub trait Stoppable {
    fn stop(&self) -> ();
}

impl Timer {
    pub fn new(interval: Duration) -> (Self, TimerStopper) {
        let (tx_stop, rx_stop): (Sender<()>, Receiver<()>) = mpsc::channel();
        let shared = Arc::new(SharedTimerState {
            tx_stop: Mutex::new(tx_stop),
            rx_stop: Mutex::new(rx_stop),
        });

        let shared_c = Arc::clone(&shared);
        (Timer { interval, shared }, TimerStopper {
            shared: shared_c,
        })
    }
}

/// Performs the internal logic to stop and then signal an update to the
/// listening thread
fn stop_timer(shared: &Arc<SharedTimerState>) -> () {
    let tx_stop = shared
        .tx_stop
        .lock()
        .expect("Could not unlock timer stop channel sender mutex: lock poisoned");
    // ignore result: if the channel was closed, then the receiver (timer
    // thread) must already have exited
    let _ = tx_stop.send(());
}

impl Stoppable for Timer {
    /// Stops the timer thread when it checks on the next tick and immediately
    /// stops iteration
    fn stop(&self) -> () { stop_timer(&self.shared); }
}

impl Stoppable for TimerStopper {
    /// Stops the timer thread and the thread blocked on the iteration
    /// immediately
    fn stop(&self) -> () { stop_timer(&self.shared); }
}

impl Drop for Timer {
    /// Stops the timer thread and the thread blocked on the iteration
    /// immediately
    fn drop(&mut self) {
        match self.shared.tx_stop.lock() {
            Err(_) => {},
            Ok(tx_stop) => {
                // ignore result: if the channel was closed, then the receiver (timer
                // thread) must already have exited
                let _ = tx_stop.send(());
            },
        }
    }
}

impl Iterator for Timer {
    type Item = ();

    /// Blocks the current thread until the next timer action, or returns None
    /// if the timer has stopped. Called by the listening thread
    fn next(&mut self) -> Option<Self::Item> {
        let rx_stop = self
            .shared
            .rx_stop
            .lock()
            .expect("Could not unlock timer stop channel receiver mutex: lock poisoned");

        // If stopping was flagged, then stop. Else, return an empty option to
        // yield to the caller and let them process the next tick
        if let Ok(_) = rx_stop.recv_timeout(self.interval) {
            None
        } else {
            Some(())
        }
    }
}

impl Clone for TimerStopper {
    fn clone(&self) -> Self {
        TimerStopper {
            shared: Arc::clone(&self.shared),
        }
    }
}
