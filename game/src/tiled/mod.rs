use crate::rkyv::{load_compressed_archive};
use crate::tiled::load::{DeserializedMapProperties, DeserializedProperties};
use alloc::borrow::Cow;
use alloc::vec::Vec;
use bevy_app::{App, Last, Plugin, Startup};
use bevy_ecs::change_detection::ResMut;
use bevy_ecs::entity::{Entities, Entity};
use bevy_ecs::event::EventReader;
use bevy_ecs::prelude::{Commands, Component, EntityCommands, IntoScheduleConfigs, Query, Trigger};
use bevy_ecs::reflect::AppTypeRegistry;
use bevy_ecs::system::{Res, RunSystemOnce};
use bevy_ecs::world::{CommandQueue, EntityWorldMut};
use bevy_platform::sync::Arc;
use bevy_playdate::asset::{AssetAsync, BitmapAsset, BitmapRef, BitmapTableAsset, ResAssetCache};
use bevy_playdate::file::{BufferedWriter, FileHandle};
use bevy_playdate::jobs::{AsyncLoadCtx, FinishedJobs, GenJobExtensions, JobFinished, JobHandle, Jobs, JobsScheduler};
use bevy_playdate::sprite::Sprite;
use bevy_reflect::Reflect;
use core::ops::Deref;
use derive_more::Deref;
use no_std_io2::io::Write;
use pd::sys::ffi::LCDBitmapFlip;
use pd_asset::tilemap::{ArchivedChunkData, ArchivedFiniteTileLayer, ArchivedImageLayer, ArchivedInfiniteTileLayer, ArchivedLayer, ArchivedLayerData, ArchivedObjectLayer, ArchivedTilemap};
use pd_asset::tileset::{ArchivedTileData, ArchivedTileset};
use pd_asset::archive::OwnedArchived;

pub mod collision;
pub mod export;
pub mod job;
mod load;
pub mod spawn;
mod types_json;

pub struct TiledPlugin;

impl Plugin for TiledPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, export_types);
        // app.add_systems(Last, load_sprite.after(Jobs::run_jobs_system));
        add_loader::<SpriteLoader>(app);
        add_loader::<MapLoader>(app);
        add_loader::<SpriteTableLoader>(app);

        app.register_type::<Static>()
            .register_type::<export::PathField>();
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Default, Hash, Component, Reflect)]
pub struct Static;

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

        load_cx.yield_next().await;

        let out = load_cx
            .with_world(move |world| {
                let app_registry = world.resource::<AppTypeRegistry>();
                let properties =
                    DeserializedMapProperties::load(data.access(), app_registry.0.read().deref());
                TiledMap { data, properties }
            })
            .await;

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

        let out = load_cx
            .with_world(move |world| {
                let app_registry = world.resource::<AppTypeRegistry>();

                let properties = data
                    .access()
                    .tiles
                    .iter()
                    .map(|tile| {
                        DeserializedProperties::load(
                            &tile.properties,
                            app_registry.0.read().deref(),
                            (),
                            false,
                        )
                    })
                    .collect();

                TiledSet { data, properties }
            })
            .await;

        Ok(out)
    }
}

#[derive(Debug, Clone)]
pub struct Map {
    map: Arc<TiledMap>,
    tilesets: Vec<Arc<TiledSet>>,
}

impl Map {
    pub fn layers(&self) -> impl Iterator<Item = Layer> {
        let map = self.map.data.access();
        map.layers
            .iter()
            .map(move |layer| Layer { map: self, layer })
    }

    pub fn get_tile_data(&self, tile: TileData) -> (&ArchivedTileData, &DeserializedProperties) {
        let map = tile.get_tilemap_idx();
        let tile_n = tile.tile_id;

        let tileset = &self.tilesets[map as usize];

        let tile_data = &tileset.data.access().tiles[tile_n as usize];
        let properties = &tileset.properties[tile_n as usize];

        (tile_data, properties)
    }
}

#[derive(Deref)]
pub struct Layer<'map> {
    map: &'map Map,
    #[deref]
    layer: &'map ArchivedLayer,
}

impl Layer<'_> {
    pub fn data(&self) -> LayerData {
        match &self.layer_data {
            ArchivedLayerData::FiniteTileLayer(layer) => LayerData::FiniteTileLayer(FiniteTileLayer {
                map: self.map,
                data: layer,
            }),
            ArchivedLayerData::InfiniteTileLayer(layer) => LayerData::InfiniteTileLayer(InfiniteTileLayer {
                map: self.map,
                data: layer,
            }),
            ArchivedLayerData::ObjectLayer(layer) => LayerData::ObjectLayer {
                map: self.map,
                data: layer,
            },
            ArchivedLayerData::ImageLayer(layer) => LayerData::ImageLayer(layer),
        }
    }

    pub fn deserialized_properties<'a>(
        &self,
        map_properties: &'a DeserializedMapProperties<true>,
    ) -> &'a DeserializedProperties {
        let id = self.id;
        map_properties.layers.get(&id.to_native()).unwrap()
    }

    // pub fn as_tile_layer(&self) -> Option<TileLayer> {
    //     if let ArchivedLayerData::FiniteTileLayer(layer) = &self.layer_data {
    //         Some(TileLayer {
    //             map: self.map,
    //             data: layer,
    //         })
    //     } else {
    //         None
    //     }
    // }
}

pub enum LayerData<'map> {
    FiniteTileLayer(FiniteTileLayer<'map>),
    InfiniteTileLayer(InfiniteTileLayer<'map>),
    ObjectLayer {
        map: &'map Map,
        data: &'map ArchivedObjectLayer,
    },
    ImageLayer(&'map ArchivedImageLayer),
}

#[derive(Deref)]
pub struct FiniteTileLayer<'map> {
    map: &'map Map,
    #[deref]
    data: &'map ArchivedFiniteTileLayer,
}

impl FiniteTileLayer<'_> {
    pub fn tiles(&self) -> impl Iterator<Item = Option<Tile>> {
        self.data.tiles.iter().map(|tile| {
            tile.as_ref().map(|t| Tile {
                map: self.map,
                tile: *t,
            })
        })
    }
}

#[derive(Deref)]
pub struct InfiniteTileLayer<'map> {
    map: &'map Map,
    #[deref]
    data: &'map ArchivedInfiniteTileLayer,
}

impl InfiniteTileLayer<'_> {
    pub fn chunks(&self) -> impl ExactSizeIterator<Item=((i32, i32), Chunk)> {
        self.data.chunk_data()
            .map(move |(pos, chunk)| (pos, Chunk {
                map: self.map,
                chunk,
            }))
    }
}

#[derive(Deref)]
pub struct Chunk<'map> {
    map: &'map Map,
    #[deref]
    chunk: &'map ArchivedChunkData,
}



#[derive(Deref)]
pub struct ObjectLayer<'map> {
    _map: &'map Map,
    #[deref]
    data: &'map ArchivedObjectLayer,
}

// impl ObjectLayer<'_> {
//     pub fn objects(&self) {
//         self.data.objects.iter()
//             .map(|obj| )
//     }
// }

pub use pd_asset::tilemap::Tile as TileData;
use crate::tiled::job::{BatchCommands};

#[derive(Deref)]
pub struct Tile<'map> {
    map: &'map Map,
    #[deref]
    tile: TileData,
}

impl Tile<'_> {
    pub fn data(&self) -> (&ArchivedTileData, &DeserializedProperties) {
        self.map.get_tile_data(self.tile)
    }
}

impl AssetAsync for Map {
    type Error = anyhow::Error;

    async fn load(load_cx: &mut AsyncLoadCtx, path: &str) -> Result<Self, Self::Error> {
        let map = load_cx.load_asset::<TiledMap>(path.into()).await?;

        let archived_map = map.data.access();
        let mut tilesets = Vec::with_capacity(archived_map.tilesets.len());
        for tileset in archived_map.tilesets.iter() {
            let tileset = load_cx
                .load_asset::<TiledSet>(Arc::from(tileset.as_str()))
                .await?;
            tilesets.push(tileset);
        }

        Ok(Self { map, tilesets })
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

pub trait AssetLoader: 'static + Send + Sync {
    type Asset: AssetAsync;

    fn on_finish_load(
        &self,
        commands: &mut BatchCommands,
        entity: Entity,
        result: Result<Arc<Self::Asset>, <<Self as AssetLoader>::Asset as AssetAsync>::Error>,
    );
}

pub fn add_loader<A: AssetLoader>(app: &mut App) {
    app.add_observer(LoadingAsset::<A>::try_load_system);
}

#[derive(Component, Default)]
pub struct Loading;

#[derive(Component)]
#[require(Loading)]
pub struct LoadingAsset<A: AssetLoader> {
    pub job: JobHandle<(), Arc<A::Asset>, <A::Asset as AssetAsync>::Error>,
    pub loader: A,
}

impl<A: AssetLoader> LoadingAsset<A> {
    pub fn try_load_system(
        trigger: Trigger<JobFinished>,
        q_loading: Query<(Entity, &Self)>,
        mut jobs: ResMut<FinishedJobs>,
        mut scheduler: ResMut<JobsScheduler>,
        mut commands: BatchCommands,
    ) {
        let job = trigger.event();
        if let Some((e, job)) = q_loading
            .iter()
            .find(|(_, loading)| loading.job.id() == job.job_id)
        {
            let result = jobs.try_claim(&job.job).expect("claim result from Jobs");
            // removes both LoadingAsset and Loading
            commands.commands().entity(e).remove_with_requires::<Self>();
            job.loader.on_finish_load(&mut commands, e, result);
        }
    }
}

#[derive(Copy, Clone)]
pub struct SpriteLoader {
    pub center: [f32; 2],
    pub z_index: i16,
    pub ignore_draw_offset: bool,
}

impl SpriteLoader {
    pub fn to_sprite(&self, image: BitmapRef) -> Sprite {
        let sprite = Sprite::new_from_bitmap(image, LCDBitmapFlip::kBitmapUnflipped);
        sprite.set_center(self.center[0], self.center[1]);
        sprite.set_z_index(self.z_index);
        sprite.set_ignores_draw_offset(self.ignore_draw_offset);

        sprite
    }
}

impl Default for SpriteLoader {
    fn default() -> Self {
        Self {
            center: [0.5; 2],
            z_index: 0,
            ignore_draw_offset: false,
        }
    }
}

impl AssetLoader for SpriteLoader {
    type Asset = BitmapAsset;

    fn on_finish_load(
        &self,
        commands: &mut BatchCommands,
        entity: Entity,
        result: Result<Arc<Self::Asset>, <<Self as AssetLoader>::Asset as AssetAsync>::Error>,
    ) {
        let image = result.unwrap();
        commands.commands().entity(entity)
            .insert(self.to_sprite(image.into()));
    }
}

pub struct MapLoader;

impl AssetLoader for MapLoader {
    type Asset = Map;

    fn on_finish_load(
        &self,
        commands: &mut BatchCommands,
        entity: Entity,
        result: Result<Arc<Self::Asset>, <<Self as AssetLoader>::Asset as AssetAsync>::Error>,
    ) {
        spawn::spawn(commands, entity, result.unwrap());
    }
}

pub trait JobCommandsExt {
    fn insert_loading_asset<A: AssetLoader>(
        &mut self,
        loader: A,
        priority: isize,
        path: impl Into<Cow<'static, str>>,
    ) -> &mut Self;
}

impl<'a> JobCommandsExt for EntityCommands<'a> {
    fn insert_loading_asset<A: AssetLoader>(
        &mut self,
        loader: A,
        priority: isize,
        path: impl Into<Cow<'static, str>>,
    ) -> &mut Self {
        let path = path.into();
        self.queue(move |mut world: EntityWorldMut| {
            if let Some(x) = world.resource::<ResAssetCache>().0.try_read().unwrap()
                .get::<A::Asset>(&path) {
                let id = world.id();
                let world = world.into_world_mut();
                world.run_system_once(move |mut commands: BatchCommands| {
                    loader.on_finish_load(&mut commands, id, Ok(x.clone()));
                });
            } else {
                let job = world
                    .resource_mut::<JobsScheduler>()
                    .load_asset::<A::Asset>(priority, path);
                world.insert(LoadingAsset { job, loader });
            }
        })
    }
}

pub struct SpriteTableLoader {
    // sprite settings
    pub sprite_loader: SpriteLoader,
    // index of bitmap in sprite
    pub index: usize,
}

impl AssetLoader for SpriteTableLoader {
    type Asset = BitmapTableAsset;

    fn on_finish_load(
        &self,
        commands: &mut BatchCommands,
        entity: Entity,
        result: Result<Arc<Self::Asset>, <<Self as AssetLoader>::Asset as AssetAsync>::Error>,
    ) {
        let table = result.unwrap();
        let image = BitmapRef::from_table(table, self.index);

        commands.commands().entity(entity).insert(self.sprite_loader.to_sprite(image));
    }
}
