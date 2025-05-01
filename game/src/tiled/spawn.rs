use crate::tiled::{JobCommandsExt, LayerData, Map, SpriteLoader, SpriteTableLoader, Static};
use alloc::string::ToString;
use alloc::vec::Vec;
use bevy_ecs::entity::Entity;
use bevy_ecs::name::Name;
use bevy_ecs::prelude::{Component, EntityCommands};
use bevy_ecs::reflect::ReflectCommandExt;
use bevy_math::{Rot2, Vec2};
use bevy_playdate::transform::{GlobalTransform, Transform};
use hashbrown::HashMap;
use tiledpd::tilemap::{ArchivedLayerCollision, ArchivedObjectShape};
use bevy_platform::sync::Arc;
use bevy_reflect::Reflect;
use itertools::Itertools;
use parry2d::na::{Isometry2, Point2, Vector2};
use parry2d::query::{Ray, RayCast, RayIntersection};
use parry2d::shape::{Compound, Polyline, Segment, SharedShape};

/// WARNING: This component is only used to keep a reference to the Arc<Map> data.
///
/// To spawn a map in, use [`Commands::insert_loading_asset`](JobCommandsExt::insert_loading_asset)
/// with [`MapLoader`](super::MapLoader).
#[derive(Component, Clone)]
pub struct MapHandle(pub Arc<Map>);

#[derive(Component, Clone)]
pub struct TileLayer {
    map: Arc<Map>,
    tiles: HashMap<[u32; 2], Entity>,
}

impl TileLayer {
    pub fn tile_at(&self, x: u32, y: u32) -> Option<Entity> {
        self.tiles.get(&[x, y]).copied()
    }
}

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

pub fn spawn(entity_commands: &mut EntityCommands, map: Arc<Map>) {
    entity_commands.insert(MapHandle(Arc::clone(&map)));
    let map_data = map.map.data.access();
    // spawn all objects and create object-id-to-entity map
    let objects = {
        let mut objects: HashMap<u32, Entity> = HashMap::new();
        let mut entity_name: Vec<(Entity, _)> = Vec::new();
        
        for layer in map.layers() {
            match layer.data() {
                LayerData::ObjectLayer { data, .. } => {
                    for obj in data.objects.iter() {
                        let id = obj.id.to_native();
                        let entity = entity_commands.commands_mut().spawn_empty().id();
                        objects.insert(id, entity);
                        // optimization, insert batch
                        entity_name.push((
                            entity,
                            (
                                Name::new(obj.name.to_string()),
                                Transform::from_xy(obj.x.to_native(), obj.y.to_native()),
                            )
                        ));
                    }
                }
                _ => {}
            }
        }
        
        entity_commands.commands_mut()
            .insert_batch(entity_name);
        
        objects
    };
    
    let mut hydrated = map.map.properties.clone().hydrate(&objects);
    let mut z_index = 0;
    
    entity_commands.with_children(|commands| {
        for layer in map.layers() {
            let mut layer_entity = commands.spawn((
                Name::new(layer.name.to_string()),
                Transform::from_xy(layer.x.to_native(), layer.y.to_native()),
                // todo: add visibility component
            ));
            let reflect = hydrated.layers.remove(&layer.id.to_native()).unwrap();

            let is_static = reflect.properties.iter()
                .any(|s| s.represents::<Static>());

            for component in reflect.properties {
                layer_entity.insert_reflect(component);
            }

            match layer.data() {
                LayerData::TileLayer(tile_layer) => {
                    let layer_collision = TileLayerCollision::from(&tile_layer.layer_collision);
                    layer_entity.insert(layer_collision);
                    
                    if let Some(image) = tile_layer.image.as_ref() {
                        z_index += 1;
                        layer_entity.insert_loading_asset(SpriteLoader {
                            center: [0.0; 2],
                            z_index,
                        }, 10, image.to_string());

                        if is_static {
                            continue;
                        }

                        let width = tile_layer.width.to_native();
                        layer_entity.with_children(|c| {
                            for (i, tile) in tile_layer.tiles().enumerate() {
                                let Some(tile) = tile else {
                                    continue;
                                };

                                // map.tilesets

                                // let i = i as u32;
                                // let [x, y] = [i % width, i / width];
                                // let [x, y] = [x * map_data.tile_width, y * map_data.tile_height];

                                let mut tile_entity = c.spawn((
                                    Name::new("Tile"),
                                ));

                                // let mut tile_entity = layer_e.
                                //     .spawn((
                                //         Name::new("Tile"),
                                //     ));

                                let (_, properties) = tile.data();
                                for property in properties.properties.iter() {
                                    tile_entity.insert_reflect(property.to_dynamic());
                                }
                            }
                        });
                    } else {
                        todo!("implement individual tile drawing")
                    }
                }
                LayerData::ObjectLayer { map, data } => {
                    for obj in data.objects.iter() {
                        // I could remove the object here,
                        // but it's all going to be dropped at once later.
                        let entity = *objects.get(&obj.id.to_native()).unwrap();
                        layer_entity.add_child(entity);
                        let mut object = layer_entity.commands_mut().entity(entity);
                        
                        let reflect = hydrated.objects.remove(&obj.id.to_native()).unwrap();
                        for property in reflect.properties {
                            object.insert_reflect(property);
                        }
                        
                        if let &ArchivedObjectShape::Tile(tile) = &obj.shape {
                            let tileset = &map.tilesets[tile.get_tilemap_idx() as usize];
                            let path = tileset.data.access().image_path.to_string();
                            
                            z_index += 1;
                            object.insert_loading_asset(SpriteTableLoader {
                                sprite_loader: SpriteLoader {
                                    z_index,
                                    ..SpriteLoader::default()
                                },
                                index: tile.tile_id as usize,
                            }, 10, path);
                        }
                    }
                }
                LayerData::ImageLayer(image_layer) => {
                    z_index += 1;
                    layer_entity
                        .insert_loading_asset(SpriteLoader {
                            center: [0.0; 2],
                            z_index,
                        }, 10, image_layer.source.to_string());
                }
            }
        }
    });
    
    
}
