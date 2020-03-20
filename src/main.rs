use std::vec::Vec;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;
use jod_thread;

mod docker;
mod collect;

const LIST_CONTAINER_INTERVAL: u32 = 1000;
const COLLECT_INTERVAL: u32 = 50;

fn main() {
    let (tx, rx): (Sender<Vec<String>>, Receiver<Vec<String>>) = mpsc::channel();
    let _update_thread: jod_thread::JoinHandle<()> = jod_thread::spawn(move || {
        docker::run(tx, LIST_CONTAINER_INTERVAL)
    });
    let _collect_thread: jod_thread::JoinHandle<()> = jod_thread::spawn(move || {
        collect::run(rx, COLLECT_INTERVAL)
    });
}
