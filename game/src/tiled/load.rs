﻿use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::ReflectComponent;
use bevy_ecs::reflect::{ReflectBundle, ReflectResource};
use bevy_reflect::prelude::ReflectDefault;
use bevy_reflect::{
    DynamicArray, DynamicEnum, DynamicStruct, DynamicTuple, DynamicTupleStruct, DynamicVariant,
    FromReflect, NamedField, PartialReflect, Reflect, ReflectMut, ReflectRef, TypeInfo,
    TypeRegistration, TypeRegistry, UnnamedField, VariantInfo, VariantType,
};
use hashbrown::HashMap;
use pd_asset::properties::{ArchivedProperties, ArchivedPropertyValue};
use pd_asset::tilemap::{ArchivedLayerData, ArchivedTilemap};
use crate::tiled::export::PathField;

#[derive(Debug, Clone)]
pub struct DeserializedMapProperties<const HYDRATED: bool = false> {
    pub map: DeserializedProperties,
    pub layers: HashMap<u32, DeserializedProperties>,
    pub objects: HashMap<u32, DeserializedProperties>,
}

impl DeserializedMapProperties<false> {
    pub(crate) fn load(
        map: &ArchivedTilemap,
        registry: &TypeRegistry,
        // load_context: &mut LoadContext<'_>,
    ) -> Self {
        let map_props = DeserializedProperties::load(&map.properties, registry, (), true);

        let mut objects = HashMap::new();
        let mut layers = HashMap::new();
        let mut to_process = Vec::from_iter(map.layers.iter());
        while let Some(layer) = to_process.pop() {
            // layer.
            layers.insert(
                layer.id.to_native(),
                DeserializedProperties::load(&layer.properties, registry, (), false),
            );
            match &layer.layer_data {
                ArchivedLayerData::ObjectLayer(object) => {
                    for object in object.objects.iter() {
                        objects.insert(
                            object.id.to_native(),
                            DeserializedProperties::load(&object.properties, registry, (), false),
                        );
                    }
                }
                // ArchivedLayerData::Group(group) => {
                //     to_process.extend(group.layers());
                // }
                ArchivedLayerData::FiniteTileLayer(_) | 
                    ArchivedLayerData::ImageLayer(_) |
                    ArchivedLayerData::InfiniteTileLayer(_) => {}
            }
        }

        // let tiles = map
        //     .tilesets
        //     .iter()
        //     .map(|s| {
        //         (
        //             s.name.clone(),
        //             s.tiles()
        //                 .map(|(id, t)| {
        //                     (
        //                         id,
        //                         DeserializedProperties::load(
        //                             &t.properties,
        //                             registry,
        //                             (),
        //                             false,
        //                         ),
        //                     )
        //                 })
        //                 .collect(),
        //         )
        //     })
        //     .collect();

        Self {
            map: map_props,
            layers,
            // tiles,
            objects,
        }
    }

    pub(crate) fn hydrate(
        mut self,
        entity_map: &HashMap<u32, Entity>,
    ) -> DeserializedMapProperties<true> {
        self.map.hydrate(entity_map);
        for (_, layer) in self.layers.iter_mut() {
            layer.hydrate(entity_map);
        }
        for (_, obj) in self.objects.iter_mut() {
            obj.hydrate(entity_map);
        }

        DeserializedMapProperties::<true> {
            map: self.map,
            layers: self.layers,
            objects: self.objects,
        }
    }
}

/// Properties for an entity deserialized from a [`Properties`]
#[derive(Debug)]
pub struct DeserializedProperties {
    pub properties: Vec<Box<dyn PartialReflect>>,
}

impl Clone for DeserializedProperties {
    fn clone(&self) -> Self {
        Self {
            properties: self.properties.iter().map(|r| r.to_dynamic()).collect(),
        }
    }
}

impl DeserializedProperties {
    pub fn load(
        properties: &ArchivedProperties,
        registry: &TypeRegistry,
        _load_cx: (),
        resources_allowed: bool,
    ) -> Self {
        let mut props: Vec<Box<dyn PartialReflect>> = Vec::new();

        for (name, property) in properties.iter() {
            let ArchivedPropertyValue::ClassValue {
                property_type,
                properties: _,
            } = &property
            else {
                // if let PropertyValue::FileValue(file) = &property {
                //     props.push(Box::new(load_cx.loader().with_unknown_type().load(file)));
                //     continue;
                // }

                println!("error deserializing property: unknown property `{name}`:`{property:?}`");
                continue;
            };

            let Some(reg) = registry.get_with_type_path(property_type) else {
                println!("error deserializing property: `{property_type}` is not registered in the TypeRegistry.");
                continue;
            };

            if reg.data::<ReflectComponent>().is_none() && reg.data::<ReflectBundle>().is_none() {
                if reg.data::<ReflectResource>().is_some() {
                    if !resources_allowed {
                        println!(
                            "error deserializing property: Resources are only allowed as map properties"
                        );
                        continue;
                    }
                } else {
                    println!("error deserializing property: type `{property_type}` is not registered as a Component, Bundle, or Resource");
                    continue;
                }
            }

            match Self::deserialize_property(property, reg, registry, &mut Some(()), None) {
                Ok(prop) => {
                    props.push(prop);
                }
                Err(e) => {
                    println!("error deserializing property: {e}");
                }
            }
        }

        Self { properties: props }
    }

    fn deserialize_named_field(
        field: &NamedField,
        properties: &ArchivedProperties,
        registration: &TypeRegistration,
        registry: &TypeRegistry,
        load_cx: &mut Option<()>,
        parent_default_value: Option<&dyn Reflect>,
    ) -> Result<Box<dyn PartialReflect>, String> {
        let mut default_value = None;
        if let Some(default) = parent_default_value {
            default_value = match default.reflect_ref() {
                ReflectRef::Struct(t) => (*t).field(field.name()).and_then(|f| f.try_as_reflect()),
                _ => None,
            };
        }

        let value;
        if let Some(pv) = properties.get(field.name()) {
            let Some(reg) = registry.get(field.type_id()) else {
                return Err(format!("type `{}` is not registered", field.type_path()));
            };
            value = Self::deserialize_property(pv, reg, registry, load_cx, default_value)?;
        } else if let Some(def) = default_value {
            // If a default value from parent is provided, use it
            value = def.to_dynamic();
        } else if let Some(def) = default_value_from_type_path(registry, field.type_path()) {
            // If no default value from parent is not provided, try to use type default()
            value = def.into_partial_reflect();
        } else {
            return Err(format!(
                "missing property `{}` on `{}` and no default value provided",
                field.name(),
                registration.type_info().type_path(),
            ));
        }
        Ok(value)
    }

    fn deserialize_unnamed_field(
        field: &UnnamedField,
        properties: &ArchivedProperties,
        registration: &TypeRegistration,
        registry: &TypeRegistry,
        load_cx: &mut Option<()>,
        parent_default_value: Option<&dyn Reflect>,
    ) -> Result<Box<dyn PartialReflect>, String> {
        let mut default_value = None;
        if let Some(default) = parent_default_value {
            default_value = match default.reflect_ref() {
                ReflectRef::TupleStruct(t) => {
                    (*t).field(field.index()).and_then(|f| f.try_as_reflect())
                }
                ReflectRef::Tuple(t) => (*t).field(field.index()).and_then(|f| f.try_as_reflect()),
                _ => None,
            };
        }

        let value;
        if let Some(pv) = properties.get(field.index().to_string().as_str()) {
            let Some(reg) = registry.get(field.type_id()) else {
                return Err(format!("type `{}` is not registered", field.type_path()));
            };
            value = Self::deserialize_property(pv, reg, registry, load_cx, default_value)?;
        } else if let Some(def) = default_value {
            // If a default value from parent is provided, use it
            value = def.to_dynamic();
        } else if let Some(default_value) =
            default_value_from_type_path(registry, field.type_path())
        {
            // If no default value from parent is not provided, try to use type default()
            value = default_value.into_partial_reflect();
        } else {
            return Err(format!(
                "missing property `{}` on `{}` and no default value found",
                field.index(),
                registration.type_info().type_path(),
            ));
        }
        Ok(value)
    }

    fn deserialize_property(
        property: &ArchivedPropertyValue,
        registration: &TypeRegistration,
        registry: &TypeRegistry,
        load_cx: &mut Option<()>,
        default_value: Option<&dyn Reflect>,
    ) -> Result<Box<dyn PartialReflect>, String> {
        // I wonder if it's possible to call FromStr for String?
        // or ToString/Display?
        use ArchivedPropertyValue as PV;
        match (
            registration.type_info().type_path(),
            property,
            registration.type_info(),
        ) {
            ("bool", PV::BoolValue(b), _) => Ok(Box::new(*b)),

            ("i8", PV::IntValue(i), _) => Ok(Box::new(i8::try_from(i.to_native()).unwrap())),
            ("i16", PV::IntValue(i), _) => Ok(Box::new(i16::try_from(i.to_native()).unwrap())),
            ("i32", PV::IntValue(i), _) => Ok(Box::new(i.to_native())),
            ("i64", PV::IntValue(i), _) => Ok(Box::new(i.to_native() as i64)),
            ("i128", PV::IntValue(i), _) => Ok(Box::new(i.to_native() as i128)),
            ("u8", PV::IntValue(i), _) => Ok(Box::new(u8::try_from(i.to_native()).unwrap())),
            ("u16", PV::IntValue(i), _) => Ok(Box::new(u16::try_from(i.to_native()).unwrap())),
            ("u32", PV::IntValue(i), _) => Ok(Box::new(u32::try_from(i.to_native()).unwrap())),
            ("u64", PV::IntValue(i), _) => Ok(Box::new(u64::try_from(i.to_native()).unwrap())),
            ("u128", PV::IntValue(i), _) => Ok(Box::new(u128::try_from(i.to_native()).unwrap())),

            ("f32", PV::FloatValue(f), _) => Ok(Box::new(f.to_native())),
            ("f64", PV::FloatValue(f), _) => Ok(Box::new(f.to_native() as f64)),
            // Shouldn't need these but it's a backup
            ("f32", PV::IntValue(i), _) => Ok(Box::new(i.to_native() as f32)),
            ("f64", PV::IntValue(i), _) => Ok(Box::new(i.to_native() as f64)),

            // ("bevy_color::color::Color", PV::ColorValue(c), _) => {
            //     Ok(Box::new(Color::srgba_u8(c.red, c.green, c.blue, c.alpha)))
            // }
            ("alloc::string::String", PV::StringValue(s), _) => Ok(Box::new(s.to_string())),
            ("char", PV::StringValue(s), _) => Ok(Box::new(s.chars().next().unwrap())),
            ("alloc::string::String", PV::FileValue(s), _) => Ok(Box::new(s.to_string())),
            ("game::tiled::export::PathField", PV::FileValue(s), _) => Ok(Box::new(PathField(s.to_string()))),
            // (a, PV::FileValue(s), _) if a.starts_with("bevy_asset::handle::Handle") => {
            //     if let Some(cx) = load_cx.as_mut() {
            //         Ok(Box::new(cx.loader().with_unknown_type().load(s)))
            //     } else {
            //         Err("No LoadContext provided: cannot load Handle<T>".to_string())
            //     }
            // }
            ("bevy_ecs::entity::Entity", PV::ObjectValue(o), _) => {
                let o = o.to_native();
                if o == 0 {
                    Err("empty object reference".to_string())
                } else {
                    Ok(Box::new(Entity::from_raw(o)))
                }
            }
            ("core::option::Option<bevy_ecs::entity::Entity>", PV::ObjectValue(o), _) => {
                let o = o.to_native();
                Ok(Box::new(Some(Entity::from_raw(o)).filter(|_| o != 0)))
            }
            (_, PV::StringValue(s), TypeInfo::Enum(info)) => {
                let Some(variant) = info.variant(s) else {
                    return Err(format!("no variant `{}` for `{}`", s, info.type_path()));
                };

                let VariantInfo::Unit(unit_info) = variant else {
                    return Err(format!(
                        "variant `{}` is not a unit variant of `{}`",
                        s,
                        info.type_path()
                    ));
                };

                let mut out = DynamicEnum::new(unit_info.name(), DynamicVariant::Unit);
                out.set_represented_type(Some(registration.type_info()));

                Ok(Box::new(out))
            }
            (_, PV::ClassValue { properties, .. }, TypeInfo::Struct(info)) => {
                let mut out = DynamicStruct::default();
                out.set_represented_type(Some(registration.type_info()));

                let tmp;
                let mut default_value = default_value;
                let default_value_from_type =
                    default_value_from_type_path(registry, info.type_path());
                if default_value_from_type.is_some() {
                    tmp = default_value_from_type.unwrap();
                    default_value = Some(tmp.as_ref());
                }

                for field in info.iter() {
                    let value = Self::deserialize_named_field(
                        field,
                        properties,
                        registration,
                        registry,
                        load_cx,
                        default_value,
                    )?;
                    out.insert_boxed(field.name(), value);
                }

                Ok(Box::new(out))
            }
            (_, PV::ClassValue { properties, .. }, TypeInfo::TupleStruct(info)) => {
                let mut out = DynamicTupleStruct::default();
                out.set_represented_type(Some(registration.type_info()));

                let tmp;
                let mut default_value = default_value;
                let default_value_from_type =
                    default_value_from_type_path(registry, info.type_path());
                if default_value_from_type.is_some() {
                    tmp = default_value_from_type.unwrap();
                    default_value = Some(tmp.as_ref());
                }

                for field in info.iter() {
                    let value = Self::deserialize_unnamed_field(
                        field,
                        properties,
                        registration,
                        registry,
                        load_cx,
                        default_value,
                    )?;
                    out.insert_boxed(value);
                }

                Ok(Box::new(out))
            }
            (_, PV::ClassValue { properties, .. }, TypeInfo::Tuple(info)) => {
                let mut out = DynamicTuple::default();
                out.set_represented_type(Some(registration.type_info()));

                let tmp;
                let mut default_value = default_value;
                let default_value_from_type =
                    default_value_from_type_path(registry, info.type_path());
                if default_value_from_type.is_some() {
                    tmp = default_value_from_type.unwrap();
                    default_value = Some(tmp.as_ref());
                }

                for field in info.iter() {
                    let value = Self::deserialize_unnamed_field(
                        field,
                        properties,
                        registration,
                        registry,
                        load_cx,
                        default_value,
                    )?;
                    out.insert_boxed(value);
                }

                Ok(Box::new(out))
            }
            (_, PV::ClassValue { properties, .. }, TypeInfo::Array(info)) => {
                let mut array = Vec::new();

                let Some(reg) = registry.get(info.item_ty().id()) else {
                    return Err(format!(
                        "type `{}` is not registered",
                        info.item_ty().path()
                    ));
                };

                for i in 0..array.capacity() {
                    let Some(pv) = properties.get(format!("[{}]", i).as_str()) else {
                        return Err(format!(
                            "missing property on `{}`: `{}`",
                            info.type_path(),
                            i
                        ));
                    };

                    let value =
                        Self::deserialize_property(pv, reg, registry, load_cx, default_value)?;

                    array.push(value);
                }

                let mut out = DynamicArray::new(array.into());
                out.set_represented_type(Some(registration.type_info()));

                Ok(Box::new(out))
            }
            (_, PV::ClassValue { properties, .. }, TypeInfo::Enum(info)) => {
                let mut out = DynamicEnum::default();
                out.set_represented_type(Some(registration.type_info()));

                let tmp;
                let mut default_value = default_value;
                let default_value_from_type =
                    default_value_from_type_path(registry, info.type_path());
                if default_value_from_type.is_some() {
                    tmp = default_value_from_type.unwrap();
                    default_value = Some(tmp.as_ref());
                }

                if let Some(PV::StringValue(variant_name)) = properties.get(":variant") {
                    if let Some(PV::ClassValue { properties, .. }) = properties.get(variant_name) {
                        let variant_out = match info.variant(variant_name) {
                            Some(VariantInfo::Struct(variant_info)) => {
                                let mut out = DynamicStruct::default();
                                for field in variant_info.iter() {
                                    let value = Self::deserialize_named_field(
                                        field,
                                        properties,
                                        registration,
                                        registry,
                                        load_cx,
                                        default_value,
                                    )?;
                                    out.insert_boxed(field.name(), value);
                                }

                                Ok(DynamicVariant::Struct(out))
                            }
                            Some(VariantInfo::Tuple(variant_info)) => {
                                let mut out = DynamicTuple::default();
                                for field in variant_info.iter() {
                                    let value = Self::deserialize_unnamed_field(
                                        field,
                                        properties,
                                        registration,
                                        registry,
                                        load_cx,
                                        default_value,
                                    )?;
                                    out.insert_boxed(value);
                                }

                                Ok(DynamicVariant::Tuple(out))
                            }
                            Some(VariantInfo::Unit(_)) => Ok(DynamicVariant::Unit),
                            None => Err(format!(
                                "`{}` enum does not contain `{}` variant",
                                info.type_path(),
                                variant_name,
                            )),
                        }?;
                        out.set_variant_with_index(
                            info.index_of(variant_name).unwrap(),
                            variant_name.to_string(),
                            variant_out,
                        );

                        return Ok(Box::new(out));
                    }
                };

                if let Some(default_value) = default_value {
                    if let ReflectRef::Enum(e) = default_value.reflect_ref() {
                        out = e.to_dynamic_enum();
                        return Ok(Box::new(out));
                    }
                }

                Err(format!(
                    "missing enum properties for `{}` and no default value provided",
                    info.type_path()
                ))
            }
            (_, PV::ClassValue { .. }, TypeInfo::List(_)) => {
                Err("lists are currently unsupported".to_string())
            }
            (_, PV::ClassValue { .. }, TypeInfo::Map(_)) => {
                Err("maps are currently unsupported".to_string())
            }
            (_, PV::ClassValue { .. }, TypeInfo::Set(_)) => {
                Err("sets are currently unsupported".to_string())
            }
            // Note: ClassValue and TypeInfo::Value is not included
            (a, b, _) => Err(format!("unable to deserialize `{a}` from {b:?}")),
        }
    }

    pub(crate) fn hydrate(&mut self, obj_entity_map: &HashMap<u32, Entity>) {
        for resource in self.properties.iter_mut() {
            hydrate(resource.as_mut(), obj_entity_map);
        }
    }
}

fn default_value_from_type_path(registry: &TypeRegistry, path: &str) -> Option<Box<dyn Reflect>> {
    registry
        .get_with_type_path(path)
        .and_then(|reg| reg.data::<ReflectDefault>().map(|v| v.default()))
}

fn object_ref(
    obj: &dyn PartialReflect,
    obj_entity_map: &HashMap<u32, Entity>,
) -> Option<Box<dyn PartialReflect>> {
    if obj.represents::<Entity>() {
        let obj = Entity::take_from_reflect(obj.to_dynamic()).unwrap();
        if let Some(&e) = obj_entity_map.get(&obj.index()) {
            Some(Box::new(e))
        } else {
            panic!(
                "error hydrating properties: missing entity for object {}",
                obj.index()
            );
        }
    } else if obj.represents::<Option<Entity>>() {
        // maybe the map get should panic actually
        Some(Box::new(
            Option::<Entity>::take_from_reflect(obj.to_dynamic())
                .unwrap()
                .and_then(|obj| obj_entity_map.get(&obj.index()).copied()),
        ))
    } else {
        None
    }
}

fn hydrate(object: &mut dyn PartialReflect, obj_entity_map: &HashMap<u32, Entity>) {
    if let Some(obj) = object_ref(object, obj_entity_map) {
        object.apply(obj.as_partial_reflect());
        return;
    }

    match object.reflect_mut() {
        ReflectMut::Struct(s) => {
            for i in 0..s.field_len() {
                hydrate(s.field_at_mut(i).unwrap(), obj_entity_map);
            }
        }
        ReflectMut::TupleStruct(s) => {
            for i in 0..s.field_len() {
                hydrate(s.field_mut(i).unwrap(), obj_entity_map);
            }
        }
        ReflectMut::Tuple(s) => {
            for i in 0..s.field_len() {
                hydrate(s.field_mut(i).unwrap(), obj_entity_map);
            }
        }
        ReflectMut::List(s) => {
            for i in 0..s.len() {
                hydrate(s.get_mut(i).unwrap(), obj_entity_map);
            }
        }
        ReflectMut::Array(s) => {
            for i in 0..s.len() {
                hydrate(s.get_mut(i).unwrap(), obj_entity_map);
            }
        }
        ReflectMut::Enum(s) => match s.variant_type() {
            VariantType::Tuple => {
                for i in 0..s.field_len() {
                    hydrate(s.field_at_mut(i).unwrap(), obj_entity_map);
                }
            }
            VariantType::Struct => {
                for i in 0..s.field_len() {
                    let name = s.name_at(i).unwrap().to_owned();
                    hydrate(s.field_mut(&name).unwrap(), obj_entity_map);
                }
            }
            _ => {}
        },
        ReflectMut::Map(s) => {
            for i in 0..s.len() {
                let (k, v) = s.get_at_mut(i).unwrap();
                if object_ref(k, obj_entity_map).is_some() {
                    panic!("Unable to hydrate a key in a map!");
                }
                hydrate(v, obj_entity_map);
            }
        }
        // Cannot hydrate a Set since it does not have a get_mut() function
        ReflectMut::Set(_) => {}
        // we don't care about any of the other values
        ReflectMut::Opaque(_) => {}
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     #[test]
//     fn print_all_properties() {
//         let map = pdtiled::Loader::new()
//             .load_tmx_map("assets/hex_map_pointy_top_odd.tmx")
//             .unwrap();
//         println!("Found map: {:?}", map.properties);
//         for layer in map.layers() {
//             println!("Found layer: '{}' = {:?}", layer.name, layer.properties);
//             if let Some(objects_layer) = layer.as_object_layer() {
//                 for object in objects_layer.objects() {
//                     println!("Found object: '{}' = {:?}", object.name, object.properties);
//                 }
//             }
//             if let Some(tiles_layer) = layer.as_tile_layer() {
//                 for x in 0..map.height {
//                     for y in 0..map.width {
//                         if let Some(tile) = tiles_layer.get_tile(x as i32, y as i32) {
//                             if let Some(t) = tile.get_tile() {
//                                 println!(
//                                     "Found tile: {}({},{})' = {:?}",
//                                     layer.name, x, y, t.properties
//                                 );
//                             }
//                         }
//                     }
//                 }
//             }
//         }
//     }
//
//     #[test]
//     fn deserialize_simple_enum() {
//         #[derive(Reflect, PartialEq, Debug)]
//         enum EnumComponent {
//             VarA,
//             VarB,
//             VarC,
//         }
//
//         let mut registry = TypeRegistry::new();
//         registry.register::<EnumComponent>();
//
//         let raw_value = EnumComponent::VarB;
//         let tiled_value = PropertyValue::StringValue("VarB".to_string());
//
//         let res = DeserializedProperties::deserialize_property(
//             tiled_value,
//             registry
//                 .get_with_type_path(EnumComponent::type_path())
//                 .unwrap(),
//             &registry,
//             &mut None,
//             None,
//         )
//             .unwrap();
//         assert!(res.represents::<EnumComponent>());
//
//         let v: Result<EnumComponent, _> = FromReflect::take_from_reflect(res);
//         assert_eq!(v.unwrap(), raw_value);
//     }
//
//     #[test]
//     fn deserialize_nested_struct() {
//         #[derive(Reflect, Default, PartialEq, Debug)]
//         #[reflect(Default)]
//         enum TestEnum {
//             VarA,
//             #[default]
//             VarB,
//             VarC,
//         }
//
//         #[derive(Reflect, PartialEq, Debug)]
//         #[reflect(Default)]
//         struct InnerStruct {
//             another_float: f64,
//             another_integer: u16,
//             another_enum: TestEnum,
//         }
//         impl Default for InnerStruct {
//             fn default() -> Self {
//                 Self {
//                     another_float: 0.,
//                     another_integer: 42,
//                     another_enum: TestEnum::VarC,
//                 }
//             }
//         }
//
//         #[derive(Component, Reflect, Default, PartialEq, Debug)]
//         #[reflect(Component, Default)]
//         struct StructComponent {
//             a_float: f32,
//             an_enum: TestEnum,
//             a_struct: InnerStruct,
//             an_integer: i32,
//         }
//
//         let mut registry = TypeRegistry::new();
//         registry.register::<TestEnum>();
//         registry.register::<InnerStruct>();
//         registry.register::<StructComponent>();
//
//         let raw_value = StructComponent::default();
//         let tiled_value = PropertyValue::ClassValue {
//             property_type: StructComponent::type_path().to_string(),
//             properties: std::collections::HashMap::from([
//                 ("a_float".to_string(), PropertyValue::FloatValue(0.)),
//                 (
//                     "an_enum".to_string(),
//                     PropertyValue::StringValue("VarB".to_string()),
//                 ),
//                 (
//                     "a_struct".to_string(),
//                     PropertyValue::ClassValue {
//                         property_type: InnerStruct::type_path().to_string(),
//                         properties: std::collections::HashMap::from([
//                             ("another_float".to_string(), PropertyValue::FloatValue(0.)),
//                             ("another_integer".to_string(), PropertyValue::IntValue(42)),
//                             (
//                                 "another_enum".to_string(),
//                                 PropertyValue::StringValue("VarC".to_string()),
//                             ),
//                         ]),
//                     },
//                 ),
//                 ("an_integer".to_string(), PropertyValue::IntValue(0)),
//             ]),
//         };
//
//         let res = DeserializedProperties::deserialize_property(
//             tiled_value,
//             registry
//                 .get_with_type_path(StructComponent::type_path())
//                 .unwrap(),
//             &registry,
//             &mut None,
//             None,
//         )
//             .unwrap();
//         assert!(res.represents::<StructComponent>());
//
//         let v: Result<StructComponent, _> = FromReflect::take_from_reflect(res);
//         assert_eq!(v.unwrap(), raw_value);
//     }
// }
