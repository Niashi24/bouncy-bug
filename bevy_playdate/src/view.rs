use bevy_reflect::prelude::ReflectDefault;
use crate::transform::{Transform, GlobalTransform};
use crate::angle::PDAngle;
use crate::sprite::{Sprite, SpriteRotation};
use bevy_app::{App, Plugin, PostUpdate};
use bevy_ecs::change_detection::*;
use bevy_ecs::prelude::*;
use bevy_math::{Affine2, Affine3A, EulerRot, Vec2};
use core::ops::Deref;
use bevy_reflect::Reflect;
use playdate::graphics::Graphics;
use playdate::sprite::draw_sprites;

pub struct ViewPlugin;

impl Plugin for ViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (
                camera_offset,
                sync_sprite_transform,
                reset_removed_camera,
            )
                .after(crate::transform::TransformSystem::TransformPropagate)
                .before(draw_sprites),
        )
            .register_type::<Camera>();
    }
}

/// Add this marker component to an entity to set it as the camera/center of the screen
#[derive(Component, Copy, Clone, PartialEq, Reflect, Default)]
#[require(Transform)]
#[reflect(Component, Default, Clone)]
pub struct Camera {
    pub offset: Vec2,
}

pub fn camera_offset(camera: Option<Single<(&Camera, &GlobalTransform)>>) {
    let Some(camera) = camera else { return };
    let (camera, transform) = camera.into_inner();

    let mut pos = transform.0;
    pos += Vec2::new(-200.0, -120.0);
    pos += camera.offset;

    Graphics::Default().set_draw_offset(-pos.x as i32, -pos.y as i32);
}

pub fn reset_removed_camera(
    camera: Option<Single<Ref<GlobalTransform>, With<Camera>>>,
    mut removed_components: RemovedComponents<Camera>,
) {
    if removed_components.read().count() > 0 && camera.is_none() {
        Graphics::Default().set_draw_offset(0, 0);
    }
}

pub fn sync_sprite_transform(
    mut q_sprite: Query<(&GlobalTransform, &mut Sprite), Or<(Changed<GlobalTransform>, Added<Sprite>)>>,
) {
    for (transform, mut spr) in q_sprite.iter_mut() {
        spr.move_to(transform.x, transform.y);
    }
}

// pub fn set_sprite_affine(sprite: &mut Sprite, spr_rot: &SpriteRotation, affine: Affine3A) {
//     let (_scale, rot, trans) = affine.to_scale_rotation_translation();
// 
//     let angle_math: f32 = rot.to_euler(EulerRot::ZYX).0;
//     // dbg!(angle);
//     // todo: replace with PDAngle::from_proper
//     let angle: PDAngle = 90.0 - angle_math.to_degrees();
//     // let bitmap = spr_rot.sample_rotation(sprite, angle);
//     // sprite.set_bitmap(bitmap);
// 
//     if let Some(rotated_info) = spr_rot.is_rotated() {
//         let center = Vec2::from(rotated_info.center);
// 
//         let new_center =
//             rotate_around_point(center, Vec2::splat(0.5), angle_math).clamp(Vec2::ZERO, Vec2::ONE);
// 
//         // println!("{} {}: {}", center, angle_math.to_degrees(), new_center);
// 
//         sprite.set_center(new_center.x, new_center.y);
//     }
// 
//     sprite.move_to(trans.x, trans.y);
// }
// 
// fn rotate_around_point(p: Vec2, anchor: Vec2, angle: f32) -> Vec2 {
//     let (sin, cos) = bevy_math::ops::sin_cos(angle);
//     Vec2::new(
//         cos * (p.x - anchor.x) - sin * (p.y - anchor.y) + anchor.x,
//         sin * (p.x - anchor.x) + cos * (p.y - anchor.y) + anchor.y,
//     )
// }
