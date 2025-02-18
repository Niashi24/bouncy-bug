#![no_std]

extern crate alloc;

#[macro_use]
extern crate playdate as pd;
pub mod game;
pub mod tiled;

use bevy_app::App;
use bevy_ecs::reflect::AppTypeRegistry;
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
    
    let _ = TYPE_REGISTRY.lock().unwrap().insert(app.world().get_resource::<AppTypeRegistry>().unwrap().0.clone());
    
    app
}

use bevy_platform_support::sync::Mutex;
use bevy_reflect::TypeRegistryArc;

pub static TYPE_REGISTRY: Mutex<Option<TypeRegistryArc>> = Mutex::new(None);


// Needed for debug build, absolutely optional
ll_symbols!();
