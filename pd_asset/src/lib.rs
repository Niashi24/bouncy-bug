#![no_std]
extern crate alloc;

pub mod dependencies;
pub mod properties;
pub mod tilemap;
pub mod tileset;
pub mod gif;
pub mod archive;

pub use dependencies::AddDependencies;

pub use rkyv;
pub use rkyv::rancor::Error as RkyvError;
