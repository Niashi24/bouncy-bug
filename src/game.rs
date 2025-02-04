use bevy_app::{App, Plugin, Startup};
use bevy_ecs::system::Commands;
use bevy_playdate::sprite::Sprite;
use pd::graphics::Graphics;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, draw_test);
    }
}

fn draw_test(mut commands: Commands) {
    commands.spawn(Sprite::new());
}
