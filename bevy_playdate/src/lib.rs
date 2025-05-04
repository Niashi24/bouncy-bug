#![feature(maybe_uninit_fill)]
#![no_std]

pub mod angle;
pub mod asset;
pub mod debug;
pub mod event;
pub mod file;
pub mod input;
pub mod jobs;
pub mod sprite;
pub mod time;
pub mod transform;
pub mod view;
pub mod visibility;

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
            transform::TransformPlugin,
            asset::AssetPlugin,
            visibility::VisibilityPlugin,
        ));
    }
}
