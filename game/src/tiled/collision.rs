use alloc::sync::Arc;
use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bevy_ecs::system::{Query, SystemParam};
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

#[derive(SystemParam)]
pub struct Collision<'w, 's> {
    layers: Query<'w, 's, (Entity, &'static TileLayerCollision, &'static GlobalTransform)>
}

impl<'w, 's> Collision<'w, 's> {
    pub fn circle_cast(
        &self,
        pos: Vec2,
        r: f32,
        vel: Vec2,
        options: ShapeCastOptions,
    ) -> Option<(Entity, ShapeCastHit)> {
        let mut out: Option<(Entity, ShapeCastHit)> = None;

        for (entity, layer, transform) in self.layers {
            if let Some(hit) = layer.circle_cast(transform, pos, r, vel, options) {
                if let Some((e, prev)) = &mut out {
                    if hit.time_of_impact < prev.time_of_impact {
                        *prev = hit;
                        *e = entity;
                    }
                } else {
                    out = Some((entity, hit));
                }
            }
        }

        out
    }
    
    pub fn overlap_circle(
        &self,
        pos: Vec2,
        r: f32,
    ) -> Option<Entity> {
        self.layers.iter()
            .find(|(e, layer, transform)| layer.overlap_circle(transform, pos, r))
            .map(|(e, _, _)| e)
    }

    pub fn raycast<'a>(
        &self,
        ray: &Ray,
        max_time_of_impact: f32,
    ) -> Option<(Entity, RayIntersection)> {
        let mut closest_ray: Option<(Entity, RayIntersection)> = None;
        for (entity, layer, transform) in self.layers.iter() {
            let hit = layer.raycast(transform, ray, max_time_of_impact);

            if let Some(hit) = hit {
                if let Some((e, prev)) = &mut closest_ray {
                    if hit.time_of_impact < prev.time_of_impact {
                        *prev = hit;
                        *e = entity;
                    }
                } else {
                    closest_ray = Some((entity, hit));
                }
            }
        }

        closest_ray
    }
    
    pub fn circle_cast_repeat<'a>(&'a self, pos: Vec2, dir: Vec2, r: f32, options: ShapeCastOptions) -> CastRepeat<'a, 'w, 's> {
        CastRepeat {
            collision: self,
            pos,
            dir,
            r,
            options,
            iterations_remaining: 8,
        }
    }
}

pub fn reflect_ray(dir: Vec2, normal: Vec2) -> Vec2 {
    let dot = Vec2::dot(dir, normal);
    dir - 2.0 * dot * normal
}

pub struct CastRepeat<'a, 'w, 's> {
    collision: &'a Collision<'w, 's>,
    pub pos: Vec2,
    pub dir: Vec2,
    pub r: f32,
    pub options: ShapeCastOptions,
    pub iterations_remaining: u32,
}

impl<'a, 'w, 's> Iterator for CastRepeat<'a, 'w, 's> {
    type Item = ShapeCastHit;

    fn next(&mut self) -> Option<Self::Item> {
        if self.iterations_remaining == 0 {
            println!("max iterations reached");
            return None;
        }
        
        let Some((_, next)) = self.collision.circle_cast(self.pos, self.r, self.dir, self.options) else {
            self.pos += self.dir * self.options.max_time_of_impact;
            self.options.max_time_of_impact = 0.0;
            
            return None;
        };
        if next.time_of_impact <= 0.001 {
            println!("was inside: {}", next.time_of_impact);
            return None;
        }
        let normal = Vec2::new(next.normal2.x, next.normal2.y);
        
        self.pos = self.pos + self.dir * next.time_of_impact;
        // need to move a bit away from the wall so we don't run into it on next iteration
        // todo: this correction should back in the direction 
        self.pos += normal * 1.0;
        
        self.dir = reflect_ray(self.dir, normal);
        self.options.max_time_of_impact -= next.time_of_impact;
        
        self.iterations_remaining = self.iterations_remaining.saturating_sub(1);
        
        Some(next)
    }
}

