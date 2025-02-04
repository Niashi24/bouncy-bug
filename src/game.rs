﻿use bevy_app::{App, Plugin, Startup, Update};
use bevy_ecs::system::{Commands, Res};
use bevy_playdate::DefaultPlugins;
use bevy_playdate::input::CrankInput;
use bevy_playdate::sprite::Sprite;
use pd::graphics::{draw_ellipse, draw_line, Graphics};
use pd::graphics::color::LCDColorConst;
use pd::sys::ffi::LCDColor;
use pd::sys::log::println;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DefaultPlugins);
        app.add_systems(Startup, draw_test);
        app.add_systems(Update, crank_test);
    }
}

fn draw_test(mut commands: Commands) {
    // commands.spawn(Sprite::new());
}

fn crank_test(input: Res<CrankInput>) {
    draw_line(10 + input.angle as i32, 50, 10 + input.angle as i32 + 100, 70, 5, LCDColor::XOR);
    
    draw_ellipse(100, 20, 200, 200, 5, input.angle + 10.0, input.angle - 10.0, LCDColor::XOR);
}
