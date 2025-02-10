use pd::sys::ffi::FileOptions;
use tiled::{ResourcePath, ResourceReader};
use bevy_playdate::file::FileHandle;

pub struct PDTiledReader;

impl ResourceReader for PDTiledReader {
    type Resource = FileHandle;
    type Error = no_std_io2::io::Error;

    fn read_from(&mut self, path: &ResourcePath) -> Result<Self::Resource, Self::Error> {
        FileHandle::open(path, FileOptions::kFileReadData | FileOptions::kFileRead)
    }
}
