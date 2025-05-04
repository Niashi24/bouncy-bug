use crate::dependencies::{AddDependencies, AddDependenciesMut};
use crate::properties::Properties;
use alloc::string::String;
use alloc::vec::Vec;
use hashbrown::HashSet;
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Clone, PartialEq, Debug, Archive, Deserialize, Serialize)]
#[rkyv(derive(Debug))]
pub struct Tileset {
    pub image_path: String,
    pub tiles: Vec<TileData>,
}

impl AddDependencies for ArchivedTileset {
    fn add_dependencies<'a: 'b, 'b>(&'a self, dependencies: &mut HashSet<&'b str>) {
        dependencies.insert(&self.image_path);
        for tile in self.tiles.iter() {
            tile.add_dependencies(dependencies);
        }
    }
}

impl AddDependenciesMut for Tileset {
    fn add_dependencies_mut<'a: 'b, 'b>(&'a mut self, dependencies: &mut Vec<&'b mut String>) {
        dependencies.push(&mut self.image_path);
        for tile in self.tiles.iter_mut() {
            tile.add_dependencies_mut(dependencies);
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

impl AddDependenciesMut for TileData {
    fn add_dependencies_mut<'a: 'b, 'b>(&'a mut self, dependencies: &mut Vec<&'b mut String>) {
        self.properties.add_dependencies_mut(dependencies);
    }
}
