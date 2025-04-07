mod pdtiled;

use std::fs;
use std::process::Command;
use tiled::{Properties, PropertyValue, TileLayer};
// use tiledpd::properties::PropertyValue2;
use tiledpd::properties::{ArchivedPropertyValue, PropertyValue as PVPD};
use tiledpd::tilemap::{ArchivedTilemap, Tile, Tilemap};
use crate::pdtiled::{convert_map, convert_property};

fn main() {
    let map = dbg!(tiled::Loader::new().load_tmx_map("assets/test-map.export.tmx").unwrap());
    // for layer in map.layers() {
    //     if let Some(TileLayer::Finite(finite)) = layer.as_tile_layer() {
    //         for y in 0..finite.height() {
    //             for x in 0..finite.width() {
    //                 let tile = finite.get_tile(x as i32, y as i32)
    //                     .map(|x| *x);
    //                 dbg!(tile);
    //             }
    //         }
    //     }
    // }
    // 
    // let image = image::RgbImage::new(3, 3);
    // image::imageops::rotate
    
    // let p = PropertyValue2::IntValue(3);
    // let p = PropertyValue::ClassValue {
    //     property_type: "property type test".to_string(),
    //     properties: Properties::from([
    //         ("int value test".to_string(), PropertyValue::IntValue(3)),
    //         ("inner class test".to_string(), PropertyValue::ClassValue {
    //             property_type: "inner property type".to_string(),
    //             properties: Properties::from([
    //                 ("final".to_string(), PropertyValue::ObjectValue(240))
    //             ]),
    //         })
    //     ]),
    // };
    // bitcode::encode(&p);
    // let p = convert_property(p);
    let p = convert_map(map);
    dbg!(&p);
    
    assert_eq!(unsafe { core::mem::transmute::<_, u16>(Option::<Tile>::None) }, 0);
    
    let x = tiledpd::rkyv::to_bytes::<tiledpd::RkyvError>(&p).unwrap();
    let compressed = lz4_flex::compress_prepend_size(&x);
    fs::write("test-pd-uncompressed.bin", &x).unwrap();
    fs::write("test-pd.bin", &compressed).unwrap();
    println!("compressed tilemap: {} bytes -> {} bytes (saved {} bytes)", x.len(), compressed.len(), (x.len() as isize - compressed.len() as isize));
    
    let read = fs::read("test-pd.bin").unwrap();
    let read = lz4_flex::decompress_size_prepended(&read).unwrap();
    let p = tiledpd::rkyv::access::<ArchivedTilemap, tiledpd::RkyvError>(&read).unwrap();
    // dbg!(p);
    
    // ~
}

