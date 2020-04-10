// Allow const generics feature in Rust nightly
#![feature(const_generics)]
// Use draining on BTreeSet to optimizing memory movement
#![feature(btree_drain_filter)]
#![allow(incomplete_features)]
// Allow for using booleans in match statements where it makes it more readable
#![allow(clippy::match_bool)]

// Re-export all items
pub mod cli;
pub mod collection;
pub mod polling;
pub mod shared;
pub mod shell;
pub mod timer;
pub mod util;
