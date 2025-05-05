#![feature(adt_const_params)]
#![no_std]

extern crate alloc;

#[macro_use]
extern crate playdate as pd;
pub mod game;
pub mod rkyv;
pub mod tiled;
mod state;

use crate::game::GamePlugin;
use bevy_app::App;
use bevy_playdate::DefaultPlugins;
use pd::display::Display;

pub const TARGET_REFRESH_RATE: f32 = 50.0;
pub const TARGET_FRAME_TIME: f32 = 1.0 / TARGET_REFRESH_RATE;

#[bevy_playdate::init_app]
fn init_app() -> App {
    Display::Default().set_refresh_rate(TARGET_REFRESH_RATE);

    let mut app = App::new();
    app.add_plugins(GamePlugin)
        .add_plugins(DefaultPlugins)
        .add_plugins(tiled::TiledPlugin)
        .add_plugins(bevy_playdate::jobs::JobPlugin);

    app
}

// Needed for debug build, absolutely optional
ll_symbols!();
