use alloc::string::String;
use alloc::vec::Vec;
use rkyv::{Archive, Deserialize, Serialize};
use crate::properties::Properties;

#[derive(Clone, PartialEq, Debug, Archive, Deserialize, Serialize)]
pub struct Tileset {
    pub image_path: String,
    pub tiles: Vec<TileData>
}

#[derive(Clone, PartialEq, Debug, Archive, Deserialize, Serialize)]
pub struct TileData {
    pub properties: Properties,
}
