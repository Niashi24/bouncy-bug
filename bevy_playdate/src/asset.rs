use alloc::borrow::Cow;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::any::{Any, TypeId};
use core::ops::Deref;
use bevy_app::{App, Plugin};
use bevy_ecs::prelude::Resource;
use hashbrown::HashMap;
use no_std_io2::io::Read;
use bevy_platform_support::sync::{Arc, LazyLock, Mutex, RwLock, Weak};
use playdate::println;
use crate::file::{BufferedReader, FileHandle};
use crate::jobs::AsyncLoadCtx;

pub struct AssetPlugin;

impl Plugin for AssetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ResAssetCache>();
    }
}

#[derive(Default)]
pub struct AssetCache {
    cache: HashMap<(Cow<'static, str>, TypeId), Weak<dyn Any + Send + Sync>>,
}

#[derive(Resource)]
pub struct ResAssetCache(pub &'static RwLock<AssetCache>);

impl Default for ResAssetCache {
    fn default() -> Self {
        Self(ASSET_CACHE.deref())
    }
}

pub static ASSET_CACHE: LazyLock<RwLock<AssetCache>> = LazyLock::new(|| RwLock::default());

pub trait AssetAsync: Sized + Send + Sync + Any {
    type Error: Any + Send + Sync;

    fn load(load_cx: &mut AsyncLoadCtx, path: &str) -> impl Future<Output = Result<Self, Self::Error>> + Send + Sync;
}

// SAFETY: playdate is single threaded
// unsafe impl Send for AssetCache {}
// unsafe impl Sync for AssetCache {}

impl AssetCache {

    /// Insert an asset of the given type into the given path, overwriting any asset currently there.
    #[must_use]
    pub fn insert<A: Any + Send + Sync>(&mut self, path: impl Into<Cow<'static, str>>, asset: A) -> Arc<A> {
        let path = path.into();
        let asset: Box<dyn Any + Send + Sync> = Box::new(asset);
        let asset = Arc::from(asset);
        self.cache.insert((path, TypeId::of::<A>()), Arc::downgrade(&asset));

        asset.downcast::<A>().unwrap()
    }
    
    /// Gets the asset of the given type if it exists. Panics if the asset at that location is not
    /// the correct type.
    pub fn get<A: Any + Send + Sync>(&self, path: &str) -> Option<Arc<A>> { 
        self.cache.get(&(Cow::from(path), TypeId::of::<A>()))
            .and_then(Weak::upgrade)
            .map(|x| x.downcast::<A>().unwrap())
    }
    
    pub fn debug_loaded(&self) {
        println!("asset cache contains:");
        for ((name, _), item) in &self.cache {
            if let Some(_) = item.upgrade() {
                println!("  {name}");
            } else {
                println!("  {name} (unloaded)");
            }
        }
    }
    
    /// Clears any entries in the cache to unloaded assets.
    pub fn clear_unused(&mut self) {
        self.cache.retain(|_, v| v.strong_count() > 0);
    }
}



