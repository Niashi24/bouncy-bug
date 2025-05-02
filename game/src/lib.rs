#![feature(adt_const_params)]
#![no_std]

extern crate alloc;

#[macro_use]
extern crate playdate as pd;
pub mod game;
pub mod rkyv;
pub mod tiled;

use crate::game::GamePlugin;
use bevy_app::App;
use bevy_playdate::DefaultPlugins;
use pd::display::Display;

#[bevy_playdate::init_app]
fn init_app() -> App {
    Display::Default().set_refresh_rate(50.0);

    let mut app = App::new();
    app.add_plugins(GamePlugin)
        .add_plugins(DefaultPlugins)
        .add_plugins(tiled::TiledPlugin)
        .add_plugins(bevy_playdate::jobs::JobPlugin);

    app
}

// Needed for debug build, absolutely optional
ll_symbols!();
