// This crate comprises hacks and glue required to test private functions from tests/
//
// Keep this as slim as possible.
//
// If you're caught using this outside this crates tests/, you get to clean up the mess.

use crate::stream_safe::StreamSafe;

pub fn stream_safe(s: &str) -> heapless::String<256> {
    StreamSafe::new(s.chars()).collect()
}

pub mod quick_check {
    pub use crate::quick_check::*;
}
