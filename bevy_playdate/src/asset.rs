use crate::jobs::{AsyncLoadCtx, GenJobExtensions};
use alloc::borrow::Cow;
use alloc::boxed::Box;
use alloc::vec::Vec;
use bevy_app::{App, Plugin};
use bevy_ecs::prelude::Resource;
use bevy_platform::sync::{Arc, LazyLock, RwLock, Weak};
use derive_more::derive::{From, Index};
use core::any::{Any, TypeId};
use core::ops::Index;
use derive_more::Deref;
use hashbrown::HashMap;
use playdate::graphics::api;
use playdate::graphics::bitmap::Bitmap;
use playdate::graphics::bitmap::table::BitmapTable;
use playdate::graphics::error::ApiError;
use playdate::println;
use diagnostic::dbg;

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

pub trait AssetAsync: Sized + Send + Sync + Any {
    type Error: Any + Send + Sync;

    fn load(load_cx: &mut AsyncLoadCtx, path: &str) -> impl Future<Output = Result<Self, Self::Error>> + Send + Sync;
}

#[derive(Deref, Clone)]
pub struct BitmapAsset(pub Bitmap);

// SAFETY: playdate is single threaded
unsafe impl Send for BitmapAsset {}
unsafe impl Sync for BitmapAsset {}

impl AssetAsync for BitmapAsset {
    type Error = ApiError;

    async fn load(_load_cx: &mut AsyncLoadCtx, path: &str) -> Result<Self, Self::Error> {
        Ok(BitmapAsset(Bitmap::<api::Default>::load(path)?))
    }
}

pub struct BitmapTableAsset {
    // only need this to keep ownership of images (freed on drop)
    _table: BitmapTable,
    // table isn't going to be changed ever so we can use a Boxed slice
    bitmaps: Box<[BitmapAsset]>,
}

impl Index<usize> for BitmapTableAsset {
    type Output = BitmapAsset;

    fn index(&self, index: usize) -> &Self::Output {
        &self.bitmaps[index]
    }
}

impl BitmapTableAsset {
    pub fn from_table(table: BitmapTable) -> Self {
        let mut len = 0;

        table.info::<api::Default>(Some(&mut len), None);
        
        // dbg!(len);

        let mut bitmaps = Vec::with_capacity(len as usize);
        for i in 0..len {
            let bitmap = table.get(i).unwrap();
            let bitmap = BitmapAsset(bitmap);
            bitmaps.push(bitmap);
        }

        Self {
            _table: table,
            bitmaps: bitmaps.into_boxed_slice(),
        }
    }

    pub fn get(&self, i: usize) -> Option<&BitmapAsset> {
        self.bitmaps.get(i)
    }
    
    pub fn len(&self) -> usize {
        self.bitmaps.len()
    }
}

impl AssetAsync for BitmapTableAsset {
    type Error = ApiError;

    async fn load(_load_cx: &mut AsyncLoadCtx, path: &str) -> Result<Self, Self::Error> {
        let table = BitmapTable::<api::Default>::load(path)?;
        // we can't yield here because BitmapTable isn't Send/Sync.
        // if loading the table or creating the bitmaps take too long in benchmarks,
        // we can wrap table in an intermediary step.
        // _load_cx.yield_next().await;
        
        Ok(BitmapTableAsset::from_table(table))
    }
}

#[derive(From, Clone)]
pub enum BitmapRef {
    #[from]
    Bitmap(Arc<BitmapAsset>),
    Table(Arc<BitmapTableAsset>, usize),
}

impl BitmapRef {
    pub fn from_table(table: Arc<BitmapTableAsset>, idx: usize) -> Self {
        Self::Table(table, idx)
    }

    pub fn from_bitmap(bitmap: Arc<BitmapAsset>) -> Self {
        Self::from(bitmap)
    }

    pub fn bitmap(&self) -> &BitmapAsset {
        self.as_ref()
    }
}

impl AsRef<BitmapAsset> for BitmapRef {
    fn as_ref(&self) -> &BitmapAsset {
        match self {
            BitmapRef::Bitmap(bitmap_asset) => &bitmap_asset,
            BitmapRef::Table(bitmap_table_asset, i) => &bitmap_table_asset[*i],
        }
    }
}

// SAFETY: playdate is single threaded
unsafe impl Send for BitmapTableAsset {}
unsafe impl Sync for BitmapTableAsset {}



