use crate::ASSET_PATH;
use image::{GenericImageView, RgbaImage};
use std::ops::Deref;
use std::path::PathBuf;
use tiled::{FiniteTileLayer, Layer, LayerTile, LayerTileData, LayerType, Object, PropertyValue, TileLayer, TilesetLocation};
use tiledpd::properties::PropertyValue as PVPD;
use tiledpd::tilemap::Tilemap;
use tiledpd::tilemap::{ImageLayer, Layer as LayerPD, LayerData, ObjectData, ObjectLayer, ObjectShape, Tile};
use tiledpd::tileset::{TileData, Tileset};

pub const TILEMAP_BINARY_EXT: &str = "tmb";
pub const TILESET_BINARY_EXT: &str = "tsb";

pub fn convert_map(map: tiled::Map) -> Tilemap {
    let layers = map
        .layers()
        .map(|layer| convert_layer(layer))
        .collect();
    
    let tilesets = map.tilesets()
        .iter()
        .map(|s|
            s.source.clone()
                // .with_extension(TILESET_BINARY_EXT)
                .to_string_lossy()
                .to_string()
        )
        .collect();
    
    let properties = convert_properties(map.properties);
    
    Tilemap {
        tilesets,
        layers,
        properties,
        tile_width: map.tile_width,
        tile_height: map.tile_height,
    }
}

pub fn convert_layer(layer: Layer) -> LayerPD {
    let data = layer.deref().clone();
    let layer_data = convert_layer_data(layer);
    
    LayerPD {
        name: layer.name.clone(),
        id: layer.id(),
        x: data.offset_x,
        y: data.offset_y,
        layer_data,
        properties: convert_properties(data.properties),
    }
}

pub fn convert_layer_data(main_layer: Layer) -> LayerData {
    match main_layer.layer_type() {
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
                            let mut tile = Tile::NONE;
                            if let Some(t) = layer.get_tile_data(x as i32, y as i32) {
                                tile = Some(convert_tile(*t));
                            }
                            tiles.push(tile);
                        }
                    }
                    
                    let image = render_tile_layer(layer);
                    // image.save()
                    let mut output_path = PathBuf::from(ASSET_PATH);
                    // layer.map().source.rel
                    let mut name = layer.map().source.file_stem().unwrap().to_owned();
                    name.push("-layer-");
                    name.push(main_layer.id().to_string());
                    name.push(".png");
                    output_path.push(name.clone());
                    std::fs::create_dir_all(output_path.parent().unwrap()).unwrap();
                    image.save(&output_path).unwrap();
                    
                    LayerData::TileLayer(tiledpd::tilemap::TileLayer {
                        width: layer.width(),
                        height: layer.height(),
                        tiles,
                        image: Some(name.to_string_lossy().to_string()),
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

pub fn render_tile_layer(layer: FiniteTileLayer) -> RgbaImage {
    let width = layer.map().tile_width * layer.width();
    let height = layer.map().tile_height * layer.height();
    let mut image = RgbaImage::new(width, height);
    
    for y in 0..layer.height() {
        for x in 0..layer.width() {
            if let Some(tile) = layer.get_tile(x as i32, y as i32) {
                let tile_image = render_layer_tile(tile);
                image::imageops::overlay(
                    &mut image,
                    &tile_image,
                    (x * layer.map().tile_width) as i64,
                    (y * layer.map().tile_height) as i64,
                );
            }
        }
    }
    
    image
}

pub fn render_layer_tile(tile: LayerTile) -> RgbaImage {
    // let image = tile.get_tileset().image.as_ref().unwrap();
    
    let mut image = if let Some(image) = tile.get_tileset().image.as_ref() {
        let mut image = image::open(&image.source).unwrap().to_rgba8();
        
        let tiles_x = image.width() / tile.get_tileset().tile_width;
        
        let t_x = (tile.id() % tiles_x) * tile.get_tileset().tile_width;
        let t_y = (tile.id() / tiles_x) * tile.get_tileset().tile_height;
        let cropped = image.view(t_x, t_y, tile.get_tileset().tile_width, tile.get_tileset().tile_height);
        
        cropped.to_image()
    } else {
        panic!();
    };

    if tile.flip_h {
        image = image::imageops::flip_horizontal(&image);
    }
    if tile.flip_v {
        image = image::imageops::flip_vertical(&image);
    }
    if tile.flip_d {
        image = flip_diagonal(image);
    }
    
    image
}

pub fn flip_diagonal(mut image: RgbaImage) -> RgbaImage {
    assert_eq!(image.width(), image.height());
    
    for y in 1..image.height() {
        for x in 0..y {
            let a = *image.get_pixel(x, y);
            let b = *image.get_pixel(y, x);
            *image.get_pixel_mut(x, y) = b;
            *image.get_pixel_mut(y, x) = a;
        }
    }
    
    image
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
        image_path: tileset.image.unwrap().source.to_string_lossy().to_string(),
    }
}
