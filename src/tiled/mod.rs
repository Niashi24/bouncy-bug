use alloc::string::ToString;
use core::ops::Deref;
use bevy_app::{App, Plugin, Startup};
use bevy_ecs::reflect::AppTypeRegistry;
use bevy_ecs::system::{NonSendMut, Res};
use no_std_io2::io::Write;
use portable_atomic_util::Arc;
use tiled::{DefaultResourceCache, Loader, ResourceCache, ResourcePath, Template, Tileset};
use bevy_playdate::asset::AssetCache;
use bevy_playdate::file::{BufferedWriter, FileHandle};

mod io;
pub mod loader;
mod export;
mod types_json;
mod load;

pub struct TiledPlugin;

impl Plugin for TiledPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, export_types);
    }
}

fn export_types(reg: Res<AppTypeRegistry>) {
    let path = "type-export.json";
    let file = FileHandle::write_only(path, false).unwrap();
    let mut writer = BufferedWriter::new_default(file);
    let registry = export::TypeExportRegistry::from_registry(reg.0.read().deref());
    let output = serde_json::to_vec_pretty(&registry.to_vec()).unwrap();
    writer.write_all(&output).unwrap();
    
    println!("exported types to {path}");
}

