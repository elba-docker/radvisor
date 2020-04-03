pub mod providers;

use crate::polling::providers::Provider;
use crate::shared::{IntervalWorkerContext, TargetMetadata};
use crate::timer::{Stoppable, Timer};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;

/// Thread function that updates the target list each second by default
pub fn run(
    tx: Sender<Vec<TargetMetadata>>,
    context: IntervalWorkerContext,
    provider: Box<dyn Provider>,
) {
    context.shell.status(
        "Beginning",
        format!(
            "provider polling with {} interval",
            humantime::Duration::from(context.interval)
        ),
    );

    let (timer, stop_handle) = Timer::new(context.interval);
    let has_stopped = Arc::new(AtomicBool::new(false));

    // Handle SIGINT/SIGTERMs by stopping the timer
    let mut term_rx = context.term_rx;
    let has_stopped_c = Arc::clone(&has_stopped);
    let shell_c = Arc::clone(&context.shell);
    std::thread::spawn(move || {
        term_rx.recv().unwrap();
        shell_c.status("Stopping", "polling");
        stop_handle.stop();
        has_stopped_c.store(true, Ordering::SeqCst);
    });
    // Move to mutable
    let mut provider = provider;

    for _ in timer {
        let targets: Vec<TargetMetadata> = match provider.fetch() {
            Ok(vec) => vec,
            Err(err) => {
                context
                    .shell
                    .error(format!("Could not fetch target metadata: {}", err));
                Vec::with_capacity(0)
            },
        };

        // Make sure the collection hasn't been stopped
        if !has_stopped.load(Ordering::SeqCst) {
            // If sending fails, then stop the collection thread
            if let Err(err) = tx.send(targets) {
                context.shell.error(format!(
                    "Could not send polled target data to collector thread: {}",
                    err
                ));
                break;
            }
        }
    }
}
