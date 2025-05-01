use alloc::sync::Arc;
use bevy_ecs::component::Component;
use bevy_math::Vec2;
use itertools::Itertools;
use parry2d::na::{Isometry2, Point2, Vector2};
use parry2d::query::{Ray, RayCast, RayIntersection, ShapeCastHit, ShapeCastOptions};
use parry2d::shape::{Ball, Compound, Segment, SharedShape};
use bevy_playdate::transform::GlobalTransform;
use tiledpd::tilemap::ArchivedLayerCollision;

#[derive(Component, Clone)]
pub struct TileLayerCollision(pub Compound);

impl TileLayerCollision {
    pub fn from_layer_collision(layer: &ArchivedLayerCollision) -> Self {
        Self::from(layer)
    }

    pub fn raycast(
        &self,
        transform: &GlobalTransform,
        ray: &Ray,
        max_time_of_impact: f32,
    ) -> Option<RayIntersection> {
        // let ray = Ray::new(
        //     Point2::new(pos.x - transform.x, pos.y - transform.y),
        //     Vector2::from([dir.cos, dir.sin])
        // );

        self.0.cast_ray_and_get_normal(
            &Isometry2::translation(transform.x, transform.y),
            &ray,
            max_time_of_impact,
            true
        )
    }

    pub fn raycast_many<'a>(
        layers: impl IntoIterator<Item=(&'a Self, &'a GlobalTransform)>,
        ray: &Ray,
        max_time_of_impact: f32,
    ) -> Option<RayIntersection> {
        let mut closest_ray: Option<RayIntersection> = None;
        for (layer, transform) in layers {
            let hit = layer.raycast(transform, ray, max_time_of_impact);

            if let Some(hit) = hit {
                if let Some(prev) = &mut closest_ray {
                    if hit.time_of_impact < prev.time_of_impact {
                        *prev = hit;
                    }
                } else {
                    closest_ray = Some(hit);
                }
            }
        }

        closest_ray
    }

    pub fn overlap_circle(
        &self,
        transform: &GlobalTransform,
        pos: Vec2,
        r: f32,
    ) -> bool {
        let ball = Ball::new(r);
        parry2d::query::intersection_test(
            &Isometry2::translation(pos.x, pos.y),
            &ball,
            &Isometry2::translation(transform.x, transform.y),
            &self.0
        ).unwrap()
    }

    pub fn circle_cast(
        &self,
        transform: &GlobalTransform,
        pos: Vec2,
        r: f32,
        vel: Vec2,
        options: ShapeCastOptions,
    ) -> Option<ShapeCastHit> {
        let ball = Ball::new(r);
        parry2d::query::cast_shapes(
            &Isometry2::translation(pos.x, pos.y),
            &Vector2::from([vel.x, vel.y]),
            &ball,
            &Isometry2::translation(transform.x, transform.y),
            &Vector2::zeros(),
            &self.0,
            options,
        ).unwrap()
    }

    pub fn circle_cast_many<'a>(
        layers: impl IntoIterator<Item=(&'a Self, &'a GlobalTransform)>,
        pos: Vec2,
        r: f32,
        vel: Vec2,
        options: ShapeCastOptions,
    ) -> Option<ShapeCastHit> {
        let mut out: Option<ShapeCastHit> = None;

        for (layer, transform) in layers {
            if let Some(hit) = layer.circle_cast(transform, pos, r, vel, options) {
                if let Some(prev) = &mut out {
                    if hit.time_of_impact < prev.time_of_impact {
                        *prev = hit;
                    }
                } else {
                    out = Some(hit);
                }
            }
        }

        out
    }
}

impl From<&ArchivedLayerCollision> for TileLayerCollision {
    fn from(value: &ArchivedLayerCollision) -> Self {
        Self(Compound::new(
            value.lines.iter()
                .flat_map(|line| {
                    line.iter()
                        .map(|p| Point2::new(p.0.to_native(), p.1.to_native()))
                        .tuple_windows()
                        .map(|(a, b)| Segment::new(a, b))
                }
                )
                .map(|polyline| (Isometry2::identity(), SharedShape(Arc::new(polyline))))
                .collect()
        ))
    }
}