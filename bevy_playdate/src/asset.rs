use alloc::borrow::Cow;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::any::Any;
use bevy_app::{App, Plugin};
use bevy_ecs::prelude::Resource;
use hashbrown::HashMap;
use no_std_io2::io::Read;
use bevy_platform_support::sync::{Arc, Weak};
use crate::file::{BufferedReader, FileHandle};

pub struct AssetPlugin;

impl Plugin for AssetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AssetCache>();
    }
}

pub trait Asset: Any + Send + Sync + 'static {
    fn load(reader: impl Read) -> Self;
}

#[derive(Resource, Default)]
pub struct AssetCache {
    cache: HashMap<Cow<'static, str>, Weak<dyn Any + Send + Sync>>,
}

// SAFETY: playdate is single threaded
// unsafe impl Send for AssetCache {}
// unsafe impl Sync for AssetCache {}

impl AssetCache {
    /// loads the asset at the given path and returns a Rc of that asset.
    /// If the asset has already been loaded and is still in use, returns a cloned Rc of that asset.
    ///
    /// The cache uses a [`Weak`] as it's storage so assets not in use will be discarded.
    /// If you are wanting to preload some assets, hold an extra Rc somewhere.
    #[must_use]
    pub fn load<A: Asset>(
        &mut self,
        path: impl Into<Cow<'static, str>>,
    ) -> Arc<A> {
        let path = path.into();
        if let Some(data) = self.cache.get(&path).and_then(|rc| rc.upgrade()) {
            // todo: pass it up
            return data.downcast::<A>().unwrap();
        }
        
        // todo: pass it up
        let file = FileHandle::read_only(&path).unwrap();
        let reader = BufferedReader::<_, 512>::new(file);
        // won't let me go directly to Arc but whatev
        let asset: Box<dyn Any + Send + Sync> = Box::new(A::load(reader));
        let asset = Arc::from(asset);
        self.cache.insert(path, Arc::downgrade(&asset));
        
        asset.downcast::<A>().unwrap()
    }
    
    /// Insert an asset of the given type into the given path, overwriting any asset currently there.
    ///
    /// Note that we have to take an extra pointer dereference because of limitations in
    /// `portable-atomics`'s [`Arc`].
    pub fn insert<A: Any + Send + Sync>(&mut self, path: impl Into<Cow<'static, str>>, asset: Arc<A>) {
        let path = path.into();
        
        // Arc -> Arc -> data
        // vs
        // Arc -> data
        let asset: Box<dyn Any + Send + Sync> = Box::new(asset);
        let asset: Arc<dyn Any + Send + Sync> = Arc::from(asset);
        
        // A::unsize()
        self.cache.insert(path, Arc::downgrade(&asset));
    }
    
    /// Gets the asset of the given type if it exists. Panics if the asset at that location is not
    /// the correct type.
    pub fn get<A: Any + Send + Sync>(&self, path: &str) -> Option<Arc<A>> { 
        self.cache.get(path)
            .and_then(Weak::upgrade)
            .map(|x| x.downcast::<A>().unwrap())
    }
    
    /// Clears any entries in the cache to unloaded assets.
    pub fn clear_unused(&mut self) {
        self.cache.retain(|_, v| v.strong_count() > 0);
    }
}



