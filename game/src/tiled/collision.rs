use alloc::sync::Arc;
use core::fmt::{Debug, Formatter};
use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bevy_ecs::system::{Query, SystemParam};
use bevy_math::Vec2;
use derive_more::{Display, Error};
use itertools::Itertools;
use parry2d::na::{Isometry2, Point2, Vector2};
use parry2d::query::{Contact, Ray, RayCast, RayIntersection, ShapeCastHit, ShapeCastOptions};
use parry2d::shape::{Ball, Compound, Segment, SharedShape};
use pd::sys::log::println;
use bevy_playdate::transform::GlobalTransform;
use diagnostic::dbg;
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
    
    pub fn contact(
        &self,
        transform: &GlobalTransform,
        pos: Vec2,
        r: f32,
    ) -> Option<Contact> {
        let ball = Ball::new(r);
        parry2d::query::contact(
            &Isometry2::translation(pos.x, pos.y),
            &ball,
            &Isometry2::translation(transform.x, transform.y),
            &self.0,
            0.0,
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
        self.cast_repeat(pos, dir, r, options, reflect_ray)
    }

    pub fn move_and_slide<'a>(&'a self, pos: Vec2, dir: Vec2, r: f32, options: ShapeCastOptions) -> CastRepeat<'a, 'w, 's> {
        self.cast_repeat(pos, dir, r, options, slide_to_surface)
    }
    
    pub fn contact(&self, pos: Vec2, r: f32) -> Option<(Entity, Contact)> {
        self.layers.iter()
            .filter_map(|(e, layer, transform)|
                layer.contact(transform, pos, r)
                    .map(|c| (e, c))
            )
            .next()
    }
    
    pub fn cast_repeat<'a>(&'a self, pos: Vec2, dir: Vec2, r: f32, options: ShapeCastOptions, dir_update: DirUpdate) -> CastRepeat<'a, 'w, 's> {
        CastRepeat {
            collision: self,
            pos,
            dir,
            r,
            options,
            iterations_remaining: 2,
            dir_update,
        }
    }
    
    
}

pub fn reflect_ray(dir: Vec2, normal: Vec2) -> Result<Vec2, CastRepeatEnd> {
    let dot = Vec2::dot(dir, normal);
    Ok(dir - 2.0 * dot * normal)
}

pub fn slide_to_surface(dir: Vec2, normal: Vec2) -> Result<Vec2, CastRepeatEnd> {
    let dir_n = dir.normalize_or_zero();
    // 1 degree of leeway to detect if it is a 90 degree angle
    const DEGREE_CUTOFF: f32 = 0.0001523048436;
    if dir_n.dot(normal).abs() < DEGREE_CUTOFF {
        Err(CastRepeatEnd::NinetyDegree)
    } else {        
        let out_dir = (dir_n - Vec2::dot(dir_n, normal) * normal).normalize_or_zero();
        
        // println!("{:?} + {:?} -> {:?}", dir, normal, out_dir * dir.length());
        
        Ok(out_dir * dir.length())
    }
}

pub type DirUpdate = fn(Vec2, Vec2) -> Result<Vec2, CastRepeatEnd>;

#[derive(Copy, Clone)]
pub struct CastRepeat<'a, 'w, 's> {
    collision: &'a Collision<'w, 's>,
    pub pos: Vec2,
    pub dir: Vec2,
    pub r: f32,
    pub options: ShapeCastOptions,
    pub iterations_remaining: u32,
    pub dir_update: DirUpdate,
}

#[derive(Debug, Display, Error, Copy, Clone, Eq, PartialEq)]
pub enum CastRepeatEnd {
    #[display("no collisions detected")]
    NoCollision,
    #[display("hit max iterations")]
    MaxIterations,
    #[display("hit collision at ninety degrees")]
    NinetyDegree,
    #[display("was inside an object at the time")]
    InsideObject,
}

impl<'a, 'w, 's> CastRepeat<'a, 'w, 's> {
    pub fn fire(&mut self) -> Result<ShapeCastHit, CastRepeatEnd> {
        if self.iterations_remaining == 0 {
            return Err(CastRepeatEnd::MaxIterations);
        }

        // Use slightly smaller radius so we can fit in tight gaps,
        // and avoid running into the same piece of collision twice
        let Some((_, next)) = self.collision.circle_cast(self.pos, self.r * 0.95, self.dir, self.options) else {
            self.pos += self.dir * self.options.max_time_of_impact;
            self.options.max_time_of_impact = 0.0;

            return Err(CastRepeatEnd::NoCollision);
        };

        if next.time_of_impact == 0.0 {
            return Err(CastRepeatEnd::InsideObject);
        }

        let time = next.time_of_impact;
        let mut next_pos = self.pos + self.dir * time;
        let normal = Vec2::new(next.normal2.x, next.normal2.y);
        // need to correct position because of the smaller radius we used in cast
        if let Some((_, contact)) = self.collision.contact(next_pos, self.r) {
            next_pos += contact.dist * Vec2::new(contact.normal1.x, contact.normal1.y);
        }
        
        // not sure if this is doing anything
        if next_pos.distance_squared(self.pos) < 0.01 {
            return Err(CastRepeatEnd::NinetyDegree);
        }

        self.pos = next_pos;

        let next_dir = (self.dir_update)(self.dir, normal)?;

        self.dir = next_dir;
        self.options.max_time_of_impact -= time;

        self.iterations_remaining = self.iterations_remaining.saturating_sub(1);

        Ok(next)
    }
}

impl<'a, 'w, 's> Debug for CastRepeat<'a, 'w, 's> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CastRepeat")
            .field("pos", &self.pos)
            .field("dir", &self.dir)
            .field("r", &self.r)
            .field("options", &self.options)
            .field("iterations_remaining", &self.iterations_remaining)
            .finish()
    }
}

