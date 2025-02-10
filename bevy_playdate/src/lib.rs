#![no_std]

pub mod angle;
pub mod debug;
pub mod event;
pub mod input;
pub mod jobs;
pub mod sprite;
pub mod time;
pub mod view;
pub mod file;

pub use bevy_playdate_macros::init_app;

extern crate alloc;

use bevy_app::{App, Plugin};

pub struct DefaultPlugins;

impl Plugin for DefaultPlugins {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            input::InputPlugin,
            sprite::SpritePlugin,
            time::PDTimePlugin,
            debug::DebugPlugin,
            view::ViewPlugin,
        ));
    }
}
