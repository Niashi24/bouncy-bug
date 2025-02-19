use alloc::borrow::Cow;
use alloc::string::{String, ToString};
use core::any::Any;
use core::ops::{Deref, DerefMut};
use bevy_ecs::prelude::In;
use bevy_ecs::reflect::AppTypeRegistry;
use bevy_ecs::system::{ParamSet, Res};
use genawaiter::sync::Co;
use genawaiter::sync::Gen;
use portable_atomic_util::Arc;
use tiled::{Loader, Map, ResourceCache, ResourcePath, Template, Tileset};
use bevy_playdate::asset::{Asset, AssetCache, ASSET_CACHE};
use bevy_playdate::jobs::{JobHandle, Jobs, JobsScheduler, WorkResult};
use crate::tiled::io::PDTiledReader;
use crate::tiled::load::DeserializedMapProperties;
use crate::tiled::loader::TiledLoader;
use crate::tiled::TiledMap;
use crate::TYPE_REGISTRY;

pub enum TiledLoadStage {
    LoadTilemap(Cow<'static, str>),
    LoadProperties(Arc<Map>),
}

fn load_tilemap_job(
    In(work): In<TiledLoadStage>,
    mut loader: TiledLoader,
    registry: Res<AppTypeRegistry>,
) -> WorkResult<TiledLoadStage, TiledMap, &'static str> {
    match work {
        TiledLoadStage::LoadTilemap(path) => {
            let map = loader.load_tmx_map(&path).unwrap();
            // println!("finished loading map");
            WorkResult::Continue(TiledLoadStage::LoadProperties(map))
        }
        TiledLoadStage::LoadProperties(map) => {
            let properties = DeserializedMapProperties::load(&map, registry.0.read().deref());
            // println!("finished deserializing properties");
            WorkResult::Success(TiledMap {
                map,
                properties,
            })
        }
    }
}

pub trait AsyncJob {
    type Success: Any + Send + Sync;
    type Error: Any + Send + Sync;
    
    async fn load(co: Co<()>) -> Result<Self::Success, Self::Error>;
}

// impl<F: FnMut()> AsyncJob for 

struct AssetCacheCache<'a>(&'a mut AssetCache);

impl<'a> ResourceCache for AssetCacheCache<'a> {
    fn get_tileset(&self, path: impl AsRef<ResourcePath>) -> Option<Arc<Tileset>> {
        self.0.get(path.as_ref())
    }

    fn insert_tileset(&mut self, path: impl AsRef<ResourcePath>, tileset: Arc<Tileset>) {
        self.0.insert_arc(path.as_ref().to_string(), tileset);
    }

    fn get_template(&self, path: impl AsRef<ResourcePath>) -> Option<Arc<Template>> {
        self.0.get(path.as_ref())
    }

    fn insert_template(&mut self, path: impl AsRef<ResourcePath>, template: Arc<Template>) {
        self.0.insert_arc(path.as_ref().to_string(), template);
    }
}

pub async fn load_tilemap_job_2(mut co: Co<()>, path: impl Into<Cow<'static, str>>) -> Result<Arc<TiledMap>, String> {
    const TILEMAP_SUFFIX: &'static str = "::Map";
    let path = path.into();
    let full_path = path.to_string() + TILEMAP_SUFFIX;
    
    let map: Arc<Map> = {
        let mut assets = ASSET_CACHE.lock().unwrap();
        if let Some(map) = assets.get(&full_path) {
            return Ok(map);
        } else if let Some(map) = assets.get(&path) {
            map
        } else {
            let mut loader = Loader::with_cache_and_reader(AssetCacheCache(assets.deref_mut()), PDTiledReader);
            let map = loader.load_tmx_map(&path).map_err(|s| s.to_string())?;
            assets.insert(path, map)
        }
    };
    co.yield_(()).await;
    let properties: DeserializedMapProperties = {
        let registry_guard = TYPE_REGISTRY.lock().unwrap();
        let registry = registry_guard.as_ref().unwrap();
        // yes, making this into its own variable is necessary because of weird lifetime issues
        // with the registry_guard that I do not know enough to solve elegantly
        let properties = DeserializedMapProperties::load(&map, registry.read().deref());
        properties
    };
    co.yield_(()).await;
    let tiled_map = {
        let mut assets = ASSET_CACHE.lock().unwrap();
        assets.insert(full_path, TiledMap {
            map,
            properties,
        })
    };
    
    Ok(tiled_map)
}

pub trait TiledJobExt {
    #[must_use]
    fn load_tilemap(&mut self, path: impl Into<Cow<'static, str>>) -> JobHandle<(), Arc<TiledMap>, String>;
}

impl TiledJobExt for JobsScheduler {
    #[must_use]
    fn load_tilemap(&mut self, path: impl Into<Cow<'static, str>>) -> JobHandle<(), Arc<TiledMap>, String> {
        // self.add(10, TiledLoadStage::LoadTilemap(path.into()), load_tilemap_job)
        let path = path.into();
        self.add_async(0, Gen::new(|co| load_tilemap_job_2(co, path)))
    }
}
