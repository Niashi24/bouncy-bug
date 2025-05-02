use crate::asset::{BitmapAsset, BitmapRef};
use crate::transform::Transform;
use bevy_app::{App, Plugin, PostUpdate};
use bevy_ecs::component::{Component, HookContext};
use bevy_ecs::prelude::SystemSet;
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_ecs::world::DeferredWorld;
use bevy_platform::sync::Arc;
use derive_more::Deref;
use playdate::api;
use playdate::graphics::api::Cache;
use playdate::graphics::bitmap::Bitmap;
use playdate::graphics::color::Color;
use playdate::graphics::{BitmapFlip, BitmapFlipExt, Graphics};
use playdate::sprite::{draw_sprites, Sprite as PDSprite};
use playdate::sys::traits::AsRaw;

pub struct SpritePlugin;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, SystemSet)]
pub struct SpriteSystemSet;

impl Plugin for SpritePlugin {
    fn build(&self, app: &mut App) {
        // todo: reflect component
        app.add_systems(PostUpdate, draw_sprites.in_set(SpriteSystemSet));
    }
}

#[derive(Component, Clone, Deref)]
#[component(on_add = add_to_display_list)]
#[component(on_replace = remove_from_display_list)]
#[require(Transform)]
pub struct Sprite {
    #[deref]
    spr: PDSprite,
    /// TODO: Replace with Handle
    bitmap: BitmapRef,
}

fn add_to_display_list(w: DeferredWorld, HookContext { entity: e, .. }: HookContext) {
    w.get::<Sprite>(e).unwrap().add();
}

fn remove_from_display_list(w: DeferredWorld, HookContext { entity: e, .. }: HookContext) {
    w.get::<Sprite>(e).unwrap().remove();
}

// SAFETY: The Playdate is single-threaded.
// The component trait requires Send + Sync
unsafe impl Send for Sprite {}
unsafe impl Sync for Sprite {}

pub fn empty_bitmap() -> Arc<BitmapAsset> {
    Arc::new(BitmapAsset(
        Bitmap::new(10, 10, Color::CLEAR).expect("create default empty bitmap"),
    ))
}

impl Sprite {
    /// Creates a new, empty Sprite
    pub fn new() -> Self {
        Self::new_from_bitmap(empty_bitmap().into(), BitmapFlip::Unflipped)
    }

    pub fn new_from_bitmap(bitmap: BitmapRef, flip: BitmapFlip) -> Self {
        let spr = PDSprite::new();
        spr.set_image(&bitmap.as_ref().0, flip);

        Self { spr, bitmap }
    }

    pub fn new_from_draw(
        width: i32,
        height: i32,
        bg_color: Color,
        draw_fn: impl FnOnce(Graphics<Cache>),
    ) -> Self {
        let image = Bitmap::new(width, height, bg_color).unwrap();

        unsafe {
            api!(graphics).pushContext.unwrap()(image.as_raw());
        }

        draw_fn(Graphics::Cached());

        unsafe {
            api!(graphics).popContext.unwrap()();
        }

        Self::new_from_bitmap(Arc::new(BitmapAsset(image)).into(), BitmapFlip::Unflipped)
    }

    pub fn bitmap(&self) -> BitmapRef {
        self.bitmap.clone()
    }

    pub fn set_bitmap(&mut self, bitmap: Arc<Bitmap>) {
        self.spr.set_image(&*bitmap, BitmapFlip::Unflipped);
    }

    // /// System to draw all sprites to the screen. Calls [`playdate::sprite::draw_sprites`].
    // ///
    // /// If your draw calls are not showing up, order that system after this one.
    // #[inline]
    // pub fn draw_sprites() {
    //     playdate::sprite::draw_sprites();
    // }
}

impl Default for Sprite {
    fn default() -> Self {
        Self::new()
    }
}
