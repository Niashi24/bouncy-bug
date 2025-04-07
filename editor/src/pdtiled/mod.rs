use std::ops::Deref;
use tiled::{Layer, LayerTileData, LayerType, Object, PropertyValue, TileLayer, TilesetLocation};
use tiledpd::tilemap::{ImageLayer, Layer as LayerPD, LayerData, ObjectData, ObjectLayer, ObjectShape, Tile};
use tiledpd::properties::PropertyValue as PVPD;
use tiledpd::tilemap::Tilemap;
use tiledpd::tileset::{TileData, Tileset};

pub fn convert_map(map: tiled::Map) -> Tilemap {
    let layers = map
        .layers()
        .map(|layer| convert_layer(layer))
        .collect();
    
    let tilesets = map.tilesets()
        .iter()
        .map(|s| s.source.to_string_lossy().to_string())
        .collect();
    
    let properties = convert_properties(map.properties);
    
    Tilemap {
        tilesets,
        layers,
        properties
    }
}

pub fn convert_layer(layer: Layer) -> LayerPD {
    let data = layer.deref().clone();
    let layer_data = convert_layer_data(layer.layer_type());
    
    LayerPD {
        x: data.offset_x,
        y: data.offset_y,
        layer_data,
        properties: convert_properties(data.properties),
    }
}

pub fn convert_layer_data(layer_type: LayerType) -> LayerData {
    match layer_type {
        LayerType::Image(layer) => {
            let Some(image) = layer.image.clone() else {
                panic!("image not set on layer");
            };
            
            // let 

            LayerData::ImageLayer(ImageLayer {
                source: image.source.to_string_lossy().to_string(),
                width: image.width,
                height: image.height,
            })
        }
        LayerType::Group(_) => todo!("group layer"),
        LayerType::Tiles(tiles) => {
            match tiles {
                TileLayer::Finite(layer) => {
                    let mut tiles = Vec::with_capacity((layer.width() * layer.height()) as usize);
                    for y in 0..layer.height() {
                        for x in 0..layer.width() {
                            let tile = layer.get_tile_data(x as i32, y as i32);
                            let tile = tile.map(|x| convert_tile(*x));
                            tiles.push(tile);
                        }
                    }
                    
                    LayerData::TileLayer(tiledpd::tilemap::TileLayer {
                        width: layer.width(),
                        height: layer.height(),
                        tiles,
                    })
                }
                TileLayer::Infinite(_) => unimplemented!("infinite layer"),
            }
        },
        LayerType::Objects(layer) => {
            let objects = layer.objects()
                .map(|obj| convert_object(obj))
                .collect();

            LayerData::ObjectLayer(ObjectLayer {
                objects,
            })
        },
    }
}

pub fn convert_tile(tile: LayerTileData) -> Tile {
    let id = u8::try_from(tile.id()).expect("convert tile id to u8");
    let idx = u8::try_from(tile.tileset_index()).expect("convert tileset index to u8");
    
    Tile::new(id, tile.flip_h, tile.flip_v, tile.flip_d, idx)
}

pub fn convert_object(object: Object) -> ObjectData {
    // let data = object.
    let shape = if let Some(tile) = object.tile_data() {
        let TilesetLocation::Map(idx) = tile.tileset_location() else {
            panic!("embedded tile");
        };
        let idx = *idx;
        assert!(idx < 16);
        
        ObjectShape::Tile(Tile::new(
            u8::try_from(tile.id()).expect("too many tiles in one map"),
            tile.flip_h,
            tile.flip_v,
            tile.flip_d,
            idx as u8
        ))
    } else {
        convert_object_shape(object.shape.clone())
    };
    
    ObjectData {
        id: object.id(),
        shape,
        name: object.name.clone(),
        x: object.x,
        y: object.y,
        visible: object.visible,
        properties: convert_properties(object.properties.clone()),
    }
}

pub fn convert_object_shape(shape: tiled::ObjectShape) -> ObjectShape {
    use tiled::ObjectShape as OS;
    match shape {
        OS::Rect { width, height } => ObjectShape::Rect { width, height },
        OS::Ellipse { width, height } => ObjectShape::Ellipse { width, height },
        OS::Polyline { points } => ObjectShape::Polyline { points },
        OS::Polygon { points } => ObjectShape::Polygon { points },
        OS::Point(x, y) => ObjectShape::Point(x, y),
        OS::Text { .. } => panic!("text object is unsupported"),
    }
}

pub fn convert_properties(properties: tiled::Properties) -> tiledpd::properties::Properties {
    properties
        .into_iter()
        .map(|(k, v)| (k, convert_property(v)))
        .collect()
}

pub fn convert_property(property: PropertyValue) -> PVPD {
    use PropertyValue as PV;
    match property {
        PV::BoolValue(v) => PVPD::BoolValue(v),
        PV::FloatValue(v) => PVPD::FloatValue(v),
        PV::IntValue(v) => PVPD::IntValue(v),
        PV::ColorValue(_) => panic!("color unsupported"),
        PV::StringValue(v) => PVPD::StringValue(v),
        PV::FileValue(v) => PVPD::FileValue(v),
        PV::ObjectValue(v) => PVPD::ObjectValue(v),
        PV::ClassValue { property_type, properties } => {
            let properties = properties
                .into_iter()
                .map(|(k, v)| (k, convert_property(v)))
                .collect();

            PVPD::ClassValue {
                property_type,
                properties,
            }
        }
    }
}

pub fn convert_tileset(tileset: tiled::Tileset) -> Tileset {
    let tiles = tileset.tiles()
        .map(|(i, t)| TileData {
            properties: convert_properties(t.properties.clone()),
        })
        .collect();
    
    Tileset {
        tiles,
        image_path: tileset.source.to_string_lossy().to_string(),
    }
}
