use radvisor::cli::{self, Command, Opts, RunCommand};
use radvisor::collection;
use radvisor::polling;
use radvisor::polling::providers::Provider;
use radvisor::shared::{CollectionEvent, IntervalWorkerContext};
use radvisor::shell::{self, Shell};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use bus::Bus;

/// Disable compilation on platforms other than Linux or Windows
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn target_check() {
    compile_error!(
        "rAdvisor only compiles for Linux and Windows targets. To request support for additional \
         platforms, feel free to file an issue at https://github.com/elba-kubernetes/radvisor/issues/new"
    );
}

/// Parses CLI args and runs the correct procedure depending on the subcommand
fn main() {
    // Setup human-readable panic handler
    human_panic::setup_panic!(human_panic::Metadata {
        name:     env!("CARGO_PKG_NAME").into(),
        version:  env!("CARGO_PKG_VERSION").into(),
        authors:  env!("CARGO_PKG_AUTHORS").into(),
        homepage: "https://github.com/elba-kubernetes/radvisor/issues/new".into(),
    });

    // Parse command line arguments
    let opts: Opts = cli::load();
    // Wrap the shell in an Arc so that it can be sent across threads
    let shell = Arc::new(shell::Shell::new(&opts.shell_options));

    // Exit if running on a platform other than Linux
    if cfg!(not(target_os = "linux")) {
        shell.error(
            "rAdvisor only runs on Linux due to its reliance on cgroups. \n\
             See https://github.com/elba-kubernetes/radvisor/issues/3 \
             for the tracking issue on adding support to Windows",
        );
        std::process::exit(1);
    }

    match opts.command {
        Command::Run(run_opts) => {
            run(run_opts, shell);
        },
    }
}

/// Bootstraps the two worker threads, preparing the necessary communication
/// between them
fn run(opts: RunCommand, shell: Arc<Shell>) {
    // Used to send collection events from the polling thread to the collection
    // thread
    let (tx, rx): (Sender<CollectionEvent>, Receiver<CollectionEvent>) = mpsc::channel();

    // Create the thread worker contexts using the term bus lock
    let term_bus = initialize_term_handler(Arc::clone(&shell));
    let mut term_bus_handle = term_bus.lock().unwrap();
    let polling_context = IntervalWorkerContext {
        interval: opts.polling.interval,
        term_rx:  term_bus_handle.add_rx(),
        shell:    Arc::clone(&shell),
    };
    let collection_context = IntervalWorkerContext {
        interval: opts.collection.interval,
        term_rx:  term_bus_handle.add_rx(),
        shell:    Arc::clone(&shell),
    };
    drop(term_bus_handle);

    // Unwrap the collection options from the resolved opts
    let collection_options = opts.collection.clone();

    // Spawn both threads
    let polling_thread: thread::JoinHandle<()> = thread::Builder::new()
        .name(String::from("poll"))
        .spawn(move || {
            // Resolve container metadata provider
            let mut provider: Box<dyn Provider> = opts.provider.into_impl();

            // Determine if the current process can connect to the provider source
            let provider_shell = Arc::clone(&polling_context.shell);
            if let Err(err) = provider.initialize(&opts, provider_shell) {
                let mut message = err.suggestion.clone();
                // Print the reason for the error in verbose mode
                if let Some(original) = err.original {
                    polling_context.shell.verbose(|_| {
                        let formatted = format!("\n\n{}", original);
                        message.push_str(&formatted);
                    });
                }
                polling_context.shell.error(message);
                std::process::exit(1);
            }

            polling::run(&tx, polling_context, provider)
        })
        .unwrap();
    let collection_thread: thread::JoinHandle<()> = thread::Builder::new()
        .name(String::from("collect"))
        .spawn(move || collection::run(&rx, collection_context, &collection_options))
        .unwrap();

    // Join the threads, which automatically exit upon termination
    collection_thread
        .join()
        .expect("Error: collection thread resulted in panic");
    polling_thread
        .join()
        .expect("Error: polling thread resulted in panic");
    shell.status("Exiting", "rAdvisor");
}

/// Initializes a bus that handles termination by broadcasting an empty message
/// to all worker threads
fn initialize_term_handler(shell: Arc<Shell>) -> Arc<Mutex<Bus<()>>> {
    let term_bus = Arc::new(Mutex::new(Bus::new(1)));
    let term_bus_c = Arc::clone(&term_bus);
    ctrlc::set_handler(move || handle_termination(&term_bus_c, Arc::clone(&shell)))
        .expect("Error: could not create SIGINT handler");

    term_bus
}

/// Handles program termination by broadcasting an empty message on a special
/// termination bus that each thread listens to
fn handle_termination(bus_lock: &Arc<Mutex<Bus<()>>>, shell: Arc<Shell>) -> ! {
    let mut bus = bus_lock.lock().unwrap();
    bus.broadcast(());

    // Try again to tear down the program
    thread::sleep(Duration::from_millis(2000));
    shell.warn("Could not shutdown gracefully on the first try. Trying again...");
    bus.broadcast(());
    thread::sleep(Duration::from_millis(1000));
    shell.warn("Forcibly closing; buffers may not be flushed.");
    std::process::exit(2);
}
