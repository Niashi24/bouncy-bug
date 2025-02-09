use crate::angle::PDAngle;
use crate::sprite::{Sprite, SpriteRotation};
use bevy_app::{App, Plugin, PostUpdate};
use bevy_ecs::change_detection::*;
use bevy_ecs::prelude::*;
use bevy_math::{Affine2, Affine3A, EulerRot, Vec2};
use bevy_transform::prelude::{GlobalTransform, Transform};
use core::ops::Deref;
use playdate::sprite::draw_sprites;

pub struct ViewPlugin;

impl Plugin for ViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            view_system
                .after(bevy_transform::systems::propagate_transforms)
                .after(bevy_transform::systems::sync_simple_transforms)
                .before(draw_sprites),
        );
    }
}

/// Add this marker component to an entity to set it as the camera/center of the screen
#[derive(Component, Copy, Clone, Eq, PartialEq)]
#[require(Transform)]
pub struct Camera;

#[derive(Copy, Clone, PartialEq, Component)]
pub struct CameraView(pub Affine2);

// Either camera has moved
// or single object moved
pub fn view_system(
    camera: Option<Single<Ref<GlobalTransform>, With<Camera>>>,
    mut q_sprites: Query<(Ref<GlobalTransform>, &SpriteRotation, &mut Sprite)>,
) {
    if let Some(camera_transform) = camera {
        let inv = camera_transform.affine().inverse();

        if camera_transform.deref().is_changed() {
            for (transform, rot, mut spr) in q_sprites.iter_mut() {
                let relative = inv * transform.deref().affine();
                set_sprite_affine(spr.as_mut(), rot, relative);
            }
        } else {
            for (transform, rot, mut spr) in q_sprites.iter_mut() {
                if !transform.is_changed() {
                    continue;
                }

                let relative = inv * transform.deref().affine();
                set_sprite_affine(spr.as_mut(), rot, relative);
            }
        }
    } else {
        for (transform, rot, mut spr) in q_sprites.iter_mut() {
            if !transform.is_changed() {
                continue;
            }

            set_sprite_affine(spr.as_mut(), rot, transform.affine());
        }
    }
}

pub fn set_sprite_affine(sprite: &mut Sprite, spr_rot: &SpriteRotation, affine: Affine3A) {
    let (_scale, rot, trans) = affine.to_scale_rotation_translation();

    let angle_math: f32 = rot.to_euler(EulerRot::ZYX).0;
    // dbg!(angle);
    // todo: replace with PDAngle::from_proper
    let angle: PDAngle = 90.0 - angle_math.to_degrees();
    let bitmap = spr_rot.sample_rotation(sprite, angle);
    sprite.set_bitmap(bitmap);

    if let Some(rotated_info) = spr_rot.is_rotated() {
        let center = Vec2::from(rotated_info.center);

        let new_center =
            rotate_around_point(center, Vec2::splat(0.5), angle_math).clamp(Vec2::ZERO, Vec2::ONE);

        // println!("{} {}: {}", center, angle_math.to_degrees(), new_center);

        sprite.set_center(new_center.x, new_center.y);
    }

    sprite.move_to(trans.x, trans.y);
}

fn rotate_around_point(p: Vec2, anchor: Vec2, angle: f32) -> Vec2 {
    let (sin, cos) = bevy_math::ops::sin_cos(angle);
    Vec2::new(
        cos * (p.x - anchor.x) - sin * (p.y - anchor.y) + anchor.x,
        sin * (p.x - anchor.x) + cos * (p.y - anchor.y) + anchor.y,
    )
}
