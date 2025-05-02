#![no_std]

#[cfg(all(feature = "std", feature = "pd"))]
compile_error!("can only enable either `std` or `pd`, not both.");
#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "pd")]
mod pd;
#[cfg(feature = "std")]
mod std;

#[cfg(feature = "pd")]
pub use pd::*;
#[cfg(feature = "std")]
pub use std::*;
