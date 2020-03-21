use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

/// Represents a timer that can be iterated on and will block until either stopped
/// or signalled by its worker thread to emit another tick
pub struct Timer {
    pub duration: Duration,
    shared: Arc<SharedTimerState>,
}

/// Represents a cloneable handle to stop a timer running its own worker thread
pub struct TimerStopper {
    shared: Arc<SharedTimerState>,
}

/// Shared concurrency control data structures used to synchronize a timer
struct SharedTimerState {
    stopping: AtomicBool,
    lock: Mutex<bool>,
    signal_tick: Condvar,
}

/// Represents a timer or timer-like object that can be stopped
pub trait Stoppable {
    fn stop(&self) -> ();
}

impl Timer {
    pub fn new(dur: Duration) -> (Timer, TimerStopper) {
        let shared = Arc::new(SharedTimerState {
            stopping: AtomicBool::new(false),
            lock: Mutex::new(false),
            signal_tick: Condvar::new(),
        });

        // Spawn timer thread
        let shared_c = Arc::clone(&shared);
        thread::spawn(move || {
            loop {
                thread::sleep(dur);

                // Check to see if the timer has stopped. If so, exit to free
                // up thread
                if shared_c.stopping.load(Ordering::Relaxed) {
                    break;
                }

                // Signal the receiving thread to wake up and perform the timer
                // action (without stopping)
                let mut signal = shared_c.lock.lock().unwrap();
                *signal = true;
                shared_c.signal_tick.notify_one();
            }
        });

        let shared_c = Arc::clone(&shared);
        (
            Timer {
                duration: dur,
                shared,
            },
            TimerStopper { shared: shared_c },
        )
    }
}

/// Performs the internal logic to stop and then signal an update to the listening
/// thread
fn stop_timer(shared: &SharedTimerState) -> () {
    shared.stopping.store(true, Ordering::SeqCst);
    let mut signal = shared.lock.lock().unwrap();
    *signal = true;
    shared.signal_tick.notify_one();
}

impl Stoppable for Timer {
    /// Stops the timer thread when it checks on the next tick and immediately
    /// stops iteration
    fn stop(&self) -> () {
        stop_timer(&self.shared);
    }
}

impl Stoppable for TimerStopper {
    /// Stops the timer thread when it checks on the next tick and immediately
    /// stops iteration
    fn stop(&self) -> () {
        stop_timer(&self.shared);
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        // Stop the timer thread when it checks on the next tick to let it exit
        self.stop();
    }
}

impl Iterator for Timer {
    type Item = ();

    /// Blocks the current thread until the next timer action, or returns None if
    /// the timer has stopped. Called by the listening thread
    fn next(&mut self) -> Option<Self::Item> {
        let mut next_tick = self.shared.lock.lock().unwrap();
        while !*next_tick {
            next_tick = self.shared.signal_tick.wait(next_tick).unwrap();
        }

        // If stopping was flagged, then stop. Else, return an empty option to yield
        // to the caller and let them process the next tick
        if self.shared.stopping.load(Ordering::SeqCst) {
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
