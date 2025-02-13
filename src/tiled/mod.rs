use alloc::string::ToString;
use bevy_app::{App, Plugin};
use bevy_ecs::system::NonSendMut;
use portable_atomic_util::Arc;
use tiled::{DefaultResourceCache, Loader, ResourceCache, ResourcePath, Template, Tileset};
use bevy_playdate::asset::AssetCache;

mod io;
pub mod loader;

pub struct TiledPlugin;

impl Plugin for TiledPlugin {
    fn build(&self, app: &mut App) {
        
    }
}
