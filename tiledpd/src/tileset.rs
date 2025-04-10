use alloc::string::String;
use alloc::vec::Vec;
use hashbrown::HashSet;
use rkyv::{Archive, Deserialize, Serialize};
use crate::dependencies::AddDependencies;
use crate::properties::Properties;

#[derive(Clone, PartialEq, Debug, Archive, Deserialize, Serialize)]
#[rkyv(derive(Debug))]
pub struct Tileset {
    pub image_path: String,
    pub tiles: Vec<TileData>
}

impl AddDependencies for ArchivedTileset {
    fn add_dependencies<'a: 'b, 'b>(&'a self, dependencies: &mut HashSet<&'b str>) {
        dependencies.insert(&self.image_path);
        for tile in self.tiles.iter() {
            tile.add_dependencies(dependencies);
        }
    }
}

#[derive(Clone, PartialEq, Debug, Archive, Deserialize, Serialize)]
#[rkyv(derive(Debug))]
pub struct TileData {
    pub properties: Properties,
}

impl AddDependencies for ArchivedTileData {
    fn add_dependencies<'a: 'b, 'b>(&'a self, dependencies: &mut HashSet<&'b str>) {
        self.properties.add_dependencies(dependencies);
    }
}
