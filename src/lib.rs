// Lint options
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::cognitive_complexity)]
#![deny(clippy::debug_assert_with_mut_call)]
#![deny(clippy::empty_line_after_outer_attr)]
#![deny(clippy::fallible_impl_from)]
#![deny(clippy::imprecise_flops)]
#![deny(clippy::missing_const_for_fn)]
#![deny(clippy::mutex_integer)]
#![deny(clippy::needless_borrow)]
#![deny(clippy::path_buf_push_overwrite)]
#![deny(clippy::redundant_pub_crate)]
#![deny(clippy::suboptimal_flops)]
#![deny(clippy::use_self)]
#![deny(clippy::useless_transmute)]
// Allow for using booleans in match statements where it makes it more readable
#![allow(clippy::match_bool)]
// Allow missing Errors docs
#![allow(clippy::missing_errors_doc)]
// Allow let x = match option() { } blocks
#![allow(clippy::single_match_else)]
// Allow trailing else statements
#![allow(clippy::redundant_else)]

// Re-export all items
pub mod cli;
pub mod collection;
pub mod polling;
pub mod shared;
pub mod shell;
pub mod timer;
pub mod util;
