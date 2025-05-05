use alloc::boxed::Box;
use crate::dependencies::{AddDependencies, AddDependenciesMut};
use crate::properties::Properties;
use alloc::string::String;
use alloc::vec::Vec;
use bytecheck::CheckBytes;
use core::num::NonZeroU8;
use core::ops::Deref;
use hashbrown::{HashMap, HashSet};
use rkyv::{Archive, Deserialize, Portable, Serialize};
use rkyv::option::ArchivedOption;
use rkyv::primitive::ArchivedI32;
use rkyv::tuple::ArchivedTuple2;

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
    FiniteTileLayer(FiniteTileLayer),
    InfiniteTileLayer(InfiniteTileLayer),
    ObjectLayer(ObjectLayer),
    ImageLayer(ImageLayer),
    // Group Layer
}

impl AddDependencies for ArchivedLayerData {
    fn add_dependencies<'a: 'b, 'b>(&'a self, dependencies: &mut HashSet<&'b str>) {
        match self {
            Self::FiniteTileLayer(layer) => layer.add_dependencies(dependencies),
            Self::ObjectLayer(layer) => layer.add_dependencies(dependencies),
            Self::ImageLayer(layer) => layer.add_dependencies(dependencies),
            Self::InfiniteTileLayer(layer) => layer.add_dependencies(dependencies),
        }
    }
}

impl AddDependenciesMut for LayerData {
    fn add_dependencies_mut<'a: 'b, 'b>(&'a mut self, dependencies: &mut Vec<&'b mut String>) {
        match self {
            Self::FiniteTileLayer(layer) => layer.add_dependencies_mut(dependencies),
            Self::ObjectLayer(layer) => layer.add_dependencies_mut(dependencies),
            Self::ImageLayer(layer) => layer.add_dependencies_mut(dependencies),
            Self::InfiniteTileLayer(layer) => layer.add_dependencies_mut(dependencies),
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
    Rect { width: f32, height: f32 },
    Ellipse { width: f32, height: f32 },
    Polyline { points: Vec<(f32, f32)> },
    Polygon { points: Vec<(f32, f32)> },
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
pub struct FiniteTileLayer {
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<Option<Tile>>,
    /// Optional, pre-baked image for layer.
    /// If `Some`, it will use the image as a single sprite on the Layer entity.
    /// If `None`, it will create a sprite on each tile entity.
    pub image: Option<String>,
    pub layer_collision: Option<LayerCollision>,
}

impl AddDependencies for ArchivedFiniteTileLayer {
    fn add_dependencies<'a: 'b, 'b>(&'a self, dependencies: &mut HashSet<&'b str>) {
        if let Some(image) = self.image.as_ref() {
            dependencies.insert(image);
        }
    }
}

impl AddDependenciesMut for FiniteTileLayer {
    fn add_dependencies_mut<'a: 'b, 'b>(&'a mut self, dependencies: &mut Vec<&'b mut String>) {
        if let Some(image) = self.image.as_mut() {
            dependencies.push(image);
        }
    }
}

#[derive(Clone, PartialEq, Debug, Archive, Deserialize, Serialize)]
#[rkyv(derive(Debug))]
pub struct InfiniteTileLayer {
    pub chunks: HashMap<(i32, i32), ChunkData>,
}

impl AddDependenciesMut for InfiniteTileLayer {
    fn add_dependencies_mut<'a: 'b, 'b>(&'a mut self, dependencies: &mut Vec<&'b mut String>) {
        self.chunks.iter_mut()
            .for_each(|(_, chunk)| chunk.add_dependencies_mut(dependencies));
    }
}

impl AddDependencies for ArchivedInfiniteTileLayer {
    fn add_dependencies<'a: 'b, 'b>(&'a self, dependencies: &mut HashSet<&'b str>) {
        self.chunks.iter()
            .for_each(|(_, chunk)| chunk.add_dependencies(dependencies));
    }
}

impl ArchivedInfiniteTileLayer {


    /// Obtains the tile data present at the position given.
    ///
    /// If the position given is invalid or the position is empty, this function will return [`None`].
    ///
    /// If you want to get a [`Tile`](`crate::Tile`) instead, use [`InfiniteTileLayer::get_tile()`].
    pub fn get_tile_data(&self, x: i32, y: i32) -> Option<&Tile> {
        let chunk_pos = ArchivedChunkData::tile_to_chunk_pos(x, y);
        let pos = ArchivedTuple2(ArchivedI32::from_native(chunk_pos.0), ArchivedI32::from_native(chunk_pos.1));
        self.chunks
            .get(&pos)
            .and_then(|chunk| {
                let relative_pos = (
                    x - chunk_pos.0 * ChunkData::WIDTH as i32,
                    y - chunk_pos.1 * ChunkData::HEIGHT as i32,
                );
                let chunk_index =
                    (relative_pos.0 + relative_pos.1 * ChunkData::WIDTH as i32) as usize;
                chunk.tiles.deref().get(chunk_index).map(ArchivedOption::as_ref)
            })
            .flatten()
    }

    /// Returns an iterator over only the data part of the chunks of this tile layer.
    ///
    /// In 99.99% of cases you'll want to use [`InfiniteTileLayer::chunks()`] instead; Using this method is only
    /// needed if you *only* require the tile data of the chunks (and no other utilities provided by
    /// the map-wrapped [`LayerTile`]), and you are in dire need for that extra bit of performance.
    ///
    /// This iterator doesn't have any particular order.
    #[inline]
    pub fn chunk_data(&self) -> impl ExactSizeIterator<Item = ((i32, i32), &ArchivedChunkData)> {
        self.chunks.iter().map(|(pos, chunk)| {
            let pos = (pos.0.to_native(), pos.1.to_native());
            (pos, chunk)
        })
    }
}

#[derive(Clone, PartialEq, Debug, Archive, Deserialize, Serialize)]
#[rkyv(derive(Debug))]
pub struct ChunkData {
    pub tiles: Box<[Option<Tile>; ChunkData::TILE_COUNT]>,
    pub collision: Option<LayerCollision>,
    pub image: Option<String>,
}

impl AddDependenciesMut for ChunkData {
    fn add_dependencies_mut<'a: 'b, 'b>(&'a mut self, dependencies: &mut Vec<&'b mut String>) {
        if let Some(image) = self.image.as_mut() {
            dependencies.push(image);
        }
    }
}

impl AddDependencies for ArchivedChunkData {
    fn add_dependencies<'a: 'b, 'b>(&'a self, dependencies: &mut HashSet<&'b str>) {
        if let Some(image) = self.image.as_ref() {
            dependencies.insert(image.as_str());
        }
    }
}

impl ChunkData {
    /// Infinite layer chunk width. This constant might change between versions, not counting as a
    /// breaking change.
    pub const WIDTH: u32 = 16;
    /// Infinite layer chunk height. This constant might change between versions, not counting as a
    /// breaking change.
    pub const HEIGHT: u32 = 16;
    /// Infinite layer chunk tile count. This constant might change between versions, not counting
    /// as a breaking change.
    pub const TILE_COUNT: usize = Self::WIDTH as usize * Self::HEIGHT as usize;
}

impl ArchivedChunkData {
    /// Obtains the tile data present at the position given relative to the chunk's top-left-most tile.
    ///
    /// If the position given is invalid or the position is empty, this function will return [`None`].
    ///
    /// If you want to get a [`LayerTile`](`crate::LayerTile`) instead, use [`Chunk::get_tile()`].
    pub fn get_tile_data(&self, x: i32, y: i32) -> Option<&Tile> {
        if x < ChunkData::WIDTH as i32 && y < ChunkData::HEIGHT as i32 && x >= 0 && y >= 0 {
            self.tiles[x as usize + y as usize * ChunkData::WIDTH as usize].as_ref()
        } else {
            None
        }
    }

    /// Returns the position of the chunk that contains the given tile position.
    pub fn tile_to_chunk_pos(x: i32, y: i32) -> (i32, i32) {
        (
            x / (ChunkData::WIDTH as i32),
            y / (ChunkData::HEIGHT as i32),
        )
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
