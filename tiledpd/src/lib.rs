#![no_std]
extern crate alloc;

pub mod tileset;
pub mod tilemap;
pub mod properties;
pub mod dependencies;
pub use dependencies::AddDependencies;

pub use rkyv;
pub use rkyv::rancor::Error as RkyvError;