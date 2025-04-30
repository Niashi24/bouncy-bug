use alloc::string::String;
use alloc::vec::Vec;
use core::num::NonZeroU8;
use rkyv::{Archive, Deserialize, Portable, Serialize};
use bytecheck::CheckBytes;
use hashbrown::HashSet;
use crate::dependencies::{AddDependencies, AddDependenciesMut};
use crate::properties::Properties;

#[derive(Clone, PartialEq, Debug, Archive, Deserialize, Serialize)]
#[rkyv(derive(Debug))]
pub struct Tilemap {
    pub tilesets: Vec<String>,
    pub layers: Vec<Layer>,
    pub properties: Properties,
    pub tile_width: u32,
    pub tile_height: u32,
}

impl AddDependencies for ArchivedTilemap {
    fn add_dependencies<'a: 'b, 'b>(&'a self, dependencies: &mut HashSet<&'b str>) {
        dependencies.extend(self.tilesets.iter().map(|s| s.as_str()));
        for layer in self.layers.iter() {
            layer.add_dependencies(dependencies);
        }
        self.properties.add_dependencies(dependencies);
    }
}

impl AddDependenciesMut for Tilemap {
    fn add_dependencies_mut<'a: 'b, 'b>(&'a mut self, dependencies: &mut Vec<&'b mut String>) {
        dependencies.extend(self.tilesets.iter_mut());
        for layer in self.layers.iter_mut() {
            layer.add_dependencies_mut(dependencies);
        }
        self.properties.add_dependencies_mut(dependencies);
    }
}

#[derive(Clone, PartialEq, Debug, Archive, Deserialize, Serialize)]
#[rkyv(derive(Debug))]
pub struct Layer {
    pub name: String,
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub visible: bool,
    pub layer_data: LayerData,
    pub properties: Properties,
}

impl AddDependencies for ArchivedLayer {
    fn add_dependencies<'a: 'b, 'b>(&'a self, dependencies: &mut HashSet<&'b str>) {
        self.layer_data.add_dependencies(dependencies);
        self.properties.add_dependencies(dependencies);
    }
}

impl AddDependenciesMut for Layer {
    fn add_dependencies_mut<'a: 'b, 'b>(&'a mut self, dependencies: &mut Vec<&'b mut String>) {
        self.layer_data.add_dependencies_mut(dependencies);
        self.properties.add_dependencies_mut(dependencies);
    }
}

#[derive(Clone, PartialEq, Debug, Archive, Deserialize, Serialize)]
#[rkyv(derive(Debug))]
pub enum LayerData {
    TileLayer(TileLayer),
    ObjectLayer(ObjectLayer),
    ImageLayer(ImageLayer),
    // Group Layer
}

impl AddDependencies for ArchivedLayerData {
    fn add_dependencies<'a: 'b, 'b>(&'a self, dependencies: &mut HashSet<&'b str>) {
        match self {
            Self::TileLayer(layer) => layer.add_dependencies(dependencies),
            Self::ObjectLayer(layer) => layer.add_dependencies(dependencies),
            Self::ImageLayer(layer) => layer.add_dependencies(dependencies),
        }
    }
}

impl AddDependenciesMut for LayerData {
    fn add_dependencies_mut<'a: 'b, 'b>(&'a mut self, dependencies: &mut Vec<&'b mut String>) {
        match self {
            Self::TileLayer(layer) => layer.add_dependencies_mut(dependencies),
            Self::ObjectLayer(layer) => layer.add_dependencies_mut(dependencies),
            Self::ImageLayer(layer) => layer.add_dependencies_mut(dependencies),
        }
    }
}

#[derive(Clone, PartialEq, Debug, Archive, Deserialize, Serialize)]
#[rkyv(derive(Debug))]
pub struct ObjectLayer {
    pub objects: Vec<ObjectData>,
}

impl AddDependencies for ArchivedObjectLayer {
    fn add_dependencies<'a: 'b, 'b>(&'a self, dependencies: &mut HashSet<&'b str>) {
        for object in self.objects.iter() {
            object.add_dependencies(dependencies);
        }
    }
}

impl AddDependenciesMut for ObjectLayer {
    fn add_dependencies_mut<'a: 'b, 'b>(&'a mut self, dependencies: &mut Vec<&'b mut String>) {
        for object in self.objects.iter_mut() {
            object.add_dependencies_mut(dependencies);
        }
    }
}

#[derive(Clone, PartialEq, Debug, Archive, Deserialize, Serialize)]
#[rkyv(derive(Debug))]
pub struct ObjectData {
    pub id: u32,
    pub shape: ObjectShape,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub visible: bool,
    pub properties: Properties,
}

impl AddDependencies for ArchivedObjectData {
    fn add_dependencies<'a: 'b, 'b>(&'a self, dependencies: &mut HashSet<&'b str>) {
        self.properties.add_dependencies(dependencies);
    }
}

impl AddDependenciesMut for ObjectData {
    fn add_dependencies_mut<'a: 'b, 'b>(&'a mut self, dependencies: &mut Vec<&'b mut String>) {
        self.properties.add_dependencies_mut(dependencies);
    }
}

#[derive(Clone, PartialEq, Debug, Archive, Deserialize, Serialize)]
#[rkyv(derive(Debug))]
pub enum ObjectShape {
    Tile(Tile),
    Rect {
        width: f32,
        height: f32,
    },
    Ellipse {
        width: f32,
        height: f32,
    },
    Polyline {
        points: Vec<(f32, f32)>,
    },
    Polygon {
        points: Vec<(f32, f32)>,
    },
    Point(f32, f32),
}

#[derive(Clone, PartialEq, Debug, Archive, Deserialize, Serialize)]
#[rkyv(derive(Debug))]
pub struct ImageLayer {
    /// The path for the image.
    pub source: String,
    /// The width in pixels of the image.
    pub width: i32,
    /// The height in pixels of the image.
    pub height: i32,
}

impl AddDependencies for ArchivedImageLayer {
    fn add_dependencies<'a: 'b, 'b>(&'a self, dependencies: &mut HashSet<&'b str>) {
        dependencies.insert(&self.source);
    }
}

impl AddDependenciesMut for ImageLayer {
    fn add_dependencies_mut<'a: 'b, 'b>(&'a mut self, dependencies: &mut Vec<&'b mut String>) {
        dependencies.push(&mut self.source);
    }
}

#[derive(Clone, PartialEq, Debug, Archive, Deserialize, Serialize)]
#[rkyv(derive(Debug))]
pub struct TileLayer {
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<Option<Tile>>,
    /// Optional, pre-baked image for layer.
    /// If `Some`, it will use the image as a single sprite on the Layer entity.
    /// If `None`, it will create a sprite on each tile entity. 
    pub image: Option<String>,
    pub layer_collision: LayerCollision,
}

impl AddDependencies for ArchivedTileLayer {
    fn add_dependencies<'a: 'b, 'b>(&'a self, dependencies: &mut HashSet<&'b str>) {
        if let Some(image) = self.image.as_ref() {
            dependencies.insert(image);
        }
    }
}

impl AddDependenciesMut for TileLayer {
    fn add_dependencies_mut<'a: 'b, 'b>(&'a mut self, dependencies: &mut Vec<&'b mut String>) {
        if let Some(image) = self.image.as_mut() {
            dependencies.push(image);
        }
    }
}

#[derive(Clone, PartialEq, Debug, Archive, Deserialize, Serialize)]
#[rkyv(derive(Debug))]
pub struct LayerCollision {
    // list of polyline points
    pub lines: Vec<Vec<(f32, f32)>>,
}

// TODO: Pack this into a single integer?
// #[derive(Copy, Clone, Eq, PartialEq, Debug, Archive, Deserialize, Serialize)]
// pub struct Tile {
//     pub flip_x: bool,
//     pub flip_y: bool,
//     pub rotation: Rotation,
//     pub tile_map: usize,
//     pub tile_idx: usize,
// }

#[derive(Copy, Clone, Eq, PartialEq, Archive, Deserialize, Serialize, Portable, CheckBytes)]
#[repr(C)]
#[rkyv(as = Tile)]
pub struct Tile {
    /// Id of tile in tilemap.
    pub tile_id: u8,
    /// NonZeroU8 mask containing flip x and y, rotation, and tilemap index.
    /// Bits are laid out in the following format:
    ///
    /// `TTTT XYD0`
    /// - TTTT = index of tilemap in map.
    /// - X = flipped on the x-axis
    /// - Y = flipped on the y-axis
    /// - D = flipped diagonally (along y=x)
    /// - 0 bit of `NonZeroU8` reserved for niche optimization with Option<Tile>
    mask: NonZeroU8,
}

impl Tile {
    pub const NONE: Option<Self> = unsafe { core::mem::transmute(0i16) };
    
    pub fn new(tile_id: u8, flip_x: bool, flip_y: bool, flip_d: bool, tilemap_idx: u8) -> Self {
        assert!(tilemap_idx < 16, "tilemap index must be in 0..16");

        let mut mask = (tilemap_idx & 0b1111) << 4;

        if flip_x {
            mask |= 1 << 3;
        }
        if flip_y {
            mask |= 1 << 2;
        }
        if flip_d {
            mask |= 1 << 1;
        }

        // Ensure the mask is non-zero
        mask |= 1;

        Self {
            tile_id,
            mask: NonZeroU8::new(mask).unwrap(),
        }
    }

    pub fn get_flip_x(&self) -> bool {
        (self.mask.get() >> 3) & 1 == 1
    }

    pub fn get_flip_y(&self) -> bool {
        (self.mask.get() >> 2) & 1 == 1
    }

    pub fn get_flip_d(&self) -> bool {
        (self.mask.get() >> 1) & 1 == 1
    }

    pub fn get_tilemap_idx(&self) -> u8 {
        self.mask.get() >> 4
    }

    pub fn set_flip_x(&mut self, flip: bool) {
        let mut mask = self.mask.get();
        if flip {
            mask |= 1 << 3;
        } else {
            mask &= !(1 << 3);
        }
        // SAFETY: zero bit is still set after mask
        self.mask = unsafe { NonZeroU8::new_unchecked(mask) };
    }

    pub fn set_flip_y(&mut self, flip: bool) {
        let mut mask = self.mask.get();
        if flip {
            mask |= 1 << 2;
        } else {
            mask &= !(1 << 2);
        }
        // SAFETY: zero bit is still set after mask
        self.mask = unsafe { NonZeroU8::new_unchecked(mask) };
    }

    pub fn set_flip_d(&mut self, flip: bool) {
        let mut mask = self.mask.get();
        if flip {
            mask |= 1 << 1;
        } else {
            mask &= !(1 << 1);
        }
        self.mask = NonZeroU8::new(mask).unwrap();
    }

    pub fn set_tilemap_idx(&mut self, idx: u8) {
        assert!(idx < 16, "tilemap index must be in 0..16");
        let mut mask = self.mask.get();
        mask &= 0b0000_1111; // clear bits 4-7
        mask |= (idx & 0b1111) << 4;
        self.mask = NonZeroU8::new(mask).unwrap();
    }
}

impl core::fmt::Display for Tile {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Tile(id={}, map={}, flips=[",
            self.tile_id,
            self.get_tilemap_idx()
        )?;

        if self.get_flip_x() {
            write!(f, "X")?;
        }
        if self.get_flip_y() {
            write!(f, "Y")?;
        }
        if self.get_flip_d() {
            write!(f, "D")?;
        }

        write!(f, "])")
    }
}

impl core::fmt::Debug for Tile {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self)
    }
}

#[cfg(test)]
mod test {
    use crate::tilemap::Tile;
    
    #[test]
    pub fn option_tile_same_size() {
        assert_eq!(size_of::<Tile>(), size_of::<Option<Tile>>());
    }
    
    #[test]
    pub fn tile_is_16_bits() {
        assert_eq!(size_of::<Tile>(), 2);
    }
}
