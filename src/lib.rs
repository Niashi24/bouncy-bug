#![no_std]

extern crate alloc;
#[macro_use]
extern crate playdate as pd;
mod game;

use bevy_app::App;
use bevy_playdate::DefaultPlugins;
use crate::game::GamePlugin;

#[bevy_playdate::init_app]
fn init_app() -> App {
    let mut app = App::new();
    app
        .add_plugins(GamePlugin)
        .add_plugins(DefaultPlugins);
    
    app
}

// Needed for debug build, absolutely optional
ll_symbols!();
