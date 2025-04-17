use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::future::Future;
use core::marker::PhantomData;
use core::ops::Deref;
use bevy_app::{App, Plugin, Startup};
use bevy_ecs::reflect::AppTypeRegistry;
use bevy_ecs::system::{NonSendMut, Res};
use no_std_io2::io::{Error, Read, Write};
use bevy_platform_support::sync::Arc;
// use tiled::{DefaultResourceCache, Loader, Map, ResourceCache, ResourcePath, Template, Tileset};
use bevy_playdate::asset::{AssetAsync, AssetCache, ResAssetCache};
use bevy_playdate::file::{BufferedWriter, FileHandle};
use bevy_playdate::jobs::{load_file_bytes, AsyncLoadCtx, GenJobExtensions};
use diagnostic::dbg;
use tiledpd::rkyv::api::high::HighValidator;
use tiledpd::rkyv::bytecheck::CheckBytes;
use tiledpd::rkyv::Portable;
use tiledpd::rkyv::rancor::Source;
use tiledpd::rkyv::seal::Seal;
use tiledpd::RkyvError;
use tiledpd::tilemap::{ArchivedTilemap};
use tiledpd::tileset::ArchivedTileset;
use crate::rkyv::{load_compressed_archive, OwnedArchived};
use crate::tiled::load::{DeserializedMapProperties, DeserializedProperties};

pub mod loader;
mod export;
mod types_json;
mod load;
pub mod job;

pub struct TiledPlugin;

impl Plugin for TiledPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, export_types);
    }
}

pub type TilemapData = OwnedArchived<ArchivedTilemap>;
pub type TilesetData = OwnedArchived<ArchivedTileset>;

#[derive(Debug)]
pub struct TiledMap {
    pub data: TilemapData,
    pub properties: DeserializedMapProperties,
}

impl AssetAsync for TiledMap {
    type Error = anyhow::Error;

    async fn load(load_cx: &mut AsyncLoadCtx, path: &str) -> Result<Self, Self::Error> {
        let data = load_compressed_archive::<ArchivedTilemap>(load_cx, path).await?;
        let access = data.access();
        dbg!(data.bytes().as_ptr() as usize);
        dbg!(access);
        
        load_cx.yield_next().await;

        let out = load_cx.with_world(move |world| {
            let app_registry = world.resource::<AppTypeRegistry>();
            let properties = DeserializedMapProperties::load(data.access(), app_registry.0.read().deref());
            TiledMap {
                data,
                properties
            }
        }).await;

        Ok(out)
    }
}

#[derive(Debug)]
pub struct TiledSet {
    pub data: TilesetData,
    pub properties: Vec<DeserializedProperties>,
}

impl AssetAsync for TiledSet {
    type Error = anyhow::Error;

    async fn load(load_cx: &mut AsyncLoadCtx, path: &str) -> Result<Self, Self::Error> {
        // dbg!(path.len());
        // for c in path.chars() {
        //     dbg!(c);
        // }
        let data = load_compressed_archive::<ArchivedTileset>(load_cx, path).await?;
        // let access = data.access();
        // dbg!(access);
        load_cx.yield_next().await;

        let out = load_cx.with_world(move |world| {
            let app_registry = world.resource::<AppTypeRegistry>();
            
            let properties = data.access().tiles.iter()
                .map(|tile| DeserializedProperties::load(&tile.properties, app_registry.0.read().deref(), (), false))
                .collect();
            
            TiledSet {
                data,
                properties
            }
        }).await;

        Ok(out)
    }
}

#[derive(Debug, Clone)]
pub struct Map {
    map: Arc<TiledMap>,
    tilesets: Vec<Arc<TiledSet>>,
}

impl AssetAsync for Map {
    type Error = anyhow::Error;

    async fn load(load_cx: &mut AsyncLoadCtx, path: &str) -> Result<Self, Self::Error> {
        let map = load_cx.load_asset::<TiledMap>(path.into()).await?;
        println!("here");
        println!("here");
        println!("here");
        let test = map.data.access();
        // dbg!(&test.tilesets);
        let archived_map = map.data.access();
        let mut tilesets = Vec::with_capacity(archived_map.tilesets.len());
        for tileset in archived_map.tilesets.iter() {
            // dbg!(tileset.as_str());
            let tileset = load_cx.load_asset::<TiledSet>(Arc::from(tileset.as_str())).await?;
            // dbg!(&tileset);
            tilesets.push(tileset);
        }
        
        Ok(Self {
            map,
            tilesets,
        })
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

