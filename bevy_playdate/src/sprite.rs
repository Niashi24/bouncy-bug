use crate::angle::PDAngle;
use alloc::rc::Rc;
use alloc::vec::Vec;
use bevy_app::{App, Plugin, PostUpdate};
use bevy_ecs::component::{Component, HookContext};
use bevy_ecs::prelude::require;
use bevy_ecs::world::DeferredWorld;
use bevy_transform::prelude::Transform;
use derive_more::Deref;
use playdate::api;
use playdate::graphics::api::Cache;
use playdate::graphics::bitmap::Bitmap;
use playdate::graphics::color::Color;
use playdate::graphics::{BitmapFlip, BitmapFlipExt, Graphics};
use playdate::sprite::{draw_sprites, Sprite as PDSprite};
use playdate::sys::traits::AsRaw;

pub struct SpritePlugin;

impl Plugin for SpritePlugin {
    fn build(&self, app: &mut App) {
        // todo: reflect component
        app.add_systems(PostUpdate, draw_sprites);
    }
}

#[derive(Component, Clone, Deref)]
#[component(on_add = add_to_display_list)]
#[component(on_replace = remove_from_display_list)]
#[require(Transform, SpriteRotation)]
pub struct Sprite {
    #[deref]
    spr: PDSprite,
    /// TODO: Replace with Handle
    bitmap: Rc<Bitmap>,
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

pub fn empty_bitmap() -> Rc<Bitmap> {
    Rc::new(Bitmap::new(0, 0, Color::CLEAR).expect("create default empty bitmap"))
}

impl Sprite {
    /// Creates a new, empty Sprite
    pub fn new() -> Self {
        Self::new_from_bitmap(empty_bitmap(), BitmapFlip::Unflipped)
    }

    pub fn new_from_bitmap(bitmap: Rc<Bitmap>, flip: BitmapFlip) -> Self {
        let spr = PDSprite::new();
        spr.set_image(&*bitmap, flip);

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

        Self::new_from_bitmap(Rc::new(image), BitmapFlip::Unflipped)
    }

    pub fn bitmap(&self) -> Rc<Bitmap> {
        self.bitmap.clone()
    }

    pub fn set_bitmap(&mut self, bitmap: Rc<Bitmap>) {
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

/// Controls how the sprite is shown when it is rotated.
#[derive(Component, Clone, Default)]
pub enum SpriteRotation {
    /// No rotation. Ignores any changes to angle.
    #[default]
    Ignore,
    /// Uses [`Bitmap::rotated_clone`] to redraw the bitmap when rotation changes.
    Redraw {
        /// The bitmap that is rotated.
        /// If `None`, uses the current bitmap on the sprite.
        reference: Option<Rc<Bitmap>>,
        rotated_info: RotatedInfo,
    },
    /// Precompute each [`Bitmap::rotated_clone`] in a certain number of directions.
    /// Use [`SpriteRotation::cached`] to auto-generate.
    Cached(Vec<Rc<Bitmap>>, RotatedInfo),
}

// SAFETY: The Playdate is single-threaded.
// The component trait requires Send + Sync
unsafe impl Send for SpriteRotation {}
unsafe impl Sync for SpriteRotation {}

impl SpriteRotation {
    /// Pre-computes a rotation of
    pub fn cached(sprite: &Sprite, resolution: usize) -> Self {
        let mut directions = Vec::with_capacity(resolution);
        for i in 0..resolution {
            let angle = i as f32 / resolution as f32 * 360.0;
            let rotated = sprite
                .bitmap
                .rotated_clone(angle, 1.0, 1.0)
                .expect("precompute bitmap rotated clone");
            directions.push(Rc::new(rotated));
        }

        let rotation_info = RotatedInfo {
            center: sprite.center(),
        };

        Self::Cached(directions, rotation_info)
    }

    pub fn sample_rotation(&self, sprite: &Sprite, angle: PDAngle) -> Rc<Bitmap> {
        match self {
            SpriteRotation::Ignore => sprite.bitmap.clone(),
            SpriteRotation::Redraw { reference, .. } => {
                let rotated = reference
                    .as_ref()
                    .unwrap_or(&sprite.bitmap)
                    .rotated_clone(angle, 1.0, 1.0)
                    .expect("rotate SpriteRotation::Redraw bitmap");

                Rc::new(rotated)
            }
            SpriteRotation::Cached(directions, ..) => {
                // dbg!(angle);
                let idx =
                    ((-angle + 720.0 + 90.0) % 360.0 * directions.len() as f32 / 360.0) as usize;
                directions.get(idx).unwrap_or(&empty_bitmap()).clone()
            }
        }
    }

    pub fn is_rotated(&self) -> Option<&RotatedInfo> {
        match self {
            SpriteRotation::Ignore => None,
            SpriteRotation::Redraw { rotated_info, .. } => Some(rotated_info),
            SpriteRotation::Cached(_, rotated_info) => Some(rotated_info),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct RotatedInfo {
    pub center: (f32, f32),
}
