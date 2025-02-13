use alloc::string::ToString;
use bevy_ecs::system::{ResMut, SystemParam};
use portable_atomic_util::Arc;
use tiled::{Loader, ResourceCache, ResourcePath, Template, Tileset};
use bevy_playdate::asset::AssetCache;

#[derive(SystemParam)]
pub struct TiledLoader<'w>(ResMut<'w, AssetCache>);

impl<'w> TiledLoader<'w> {
    pub fn loader(&mut self) -> Loader<super::io::PDTiledReader, &mut Self> {
        Loader::with_cache_and_reader(self, super::io::PDTiledReader)
    }
}

impl<'a, 'w> ResourceCache for &'a mut TiledLoader<'w> {
    fn get_tileset(&self, path: impl AsRef<ResourcePath>) -> Option<Arc<Tileset>> {
        self.0.get(path.as_ref())
    }

    fn insert_tileset(&mut self, path: impl AsRef<ResourcePath>, tileset: Arc<Tileset>) {
        self.0.insert(path.as_ref().to_string(), tileset);
    }

    fn get_template(&self, path: impl AsRef<ResourcePath>) -> Option<Arc<Template>> {
        self.0.get(path.as_ref())
    }

    fn insert_template(&mut self, path: impl AsRef<ResourcePath>, template: Arc<Template>) {
        self.0.insert(path.as_ref().to_string(), template);
    }
}