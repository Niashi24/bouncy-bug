#![no_std]

extern crate alloc;

#[macro_use]
extern crate playdate as pd;
pub mod game;
pub mod tiled;
pub mod rkyv;

use bevy_app::App;
use pd::display::Display;
use bevy_playdate::DefaultPlugins;
use crate::game::GamePlugin;

#[bevy_playdate::init_app]
fn init_app() -> App {
    Display::Default().set_refresh_rate(50.0);
    
    let mut app = App::new();
    app
        .add_plugins(GamePlugin)
        .add_plugins(DefaultPlugins)
        .add_plugins(tiled::TiledPlugin)
        .add_plugins(bevy_playdate::jobs::JobPlugin);
    
    app
}



// Needed for debug build, absolutely optional
ll_symbols!();
