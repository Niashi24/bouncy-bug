use alloc::borrow::Cow;
use core::ops::Deref;
use bevy_ecs::prelude::In;
use bevy_ecs::reflect::AppTypeRegistry;
use bevy_ecs::system::{ParamSet, Res};
use portable_atomic_util::Arc;
use tiled::Map;
use bevy_playdate::jobs::{JobHandle, Jobs, JobsScheduler, WorkResult};
use crate::tiled::load::DeserializedMapProperties;
use crate::tiled::loader::TiledLoader;
use crate::tiled::TiledMap;

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
            println!("finished loading map");
            WorkResult::Continue(TiledLoadStage::LoadProperties(map))
        }
        TiledLoadStage::LoadProperties(map) => {
            let properties = DeserializedMapProperties::load(&map, registry.0.read().deref());
            println!("finished deserializing properties");
            WorkResult::Success(TiledMap {
                map,
                properties,
            })
        }
    }
}

pub trait TiledJobExt {
    #[must_use]
    fn load_tilemap(&mut self, path: impl Into<Cow<'static, str>>) -> JobHandle<TiledLoadStage, TiledMap, &'static str>;
}

impl TiledJobExt for JobsScheduler {
    #[must_use]
    fn load_tilemap(&mut self, path: impl Into<Cow<'static, str>>) -> JobHandle<TiledLoadStage, TiledMap, &'static str> {
        self.add(10, TiledLoadStage::LoadTilemap(path.into()), load_tilemap_job)
    }
}
