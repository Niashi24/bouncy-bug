mod pdtiled;

use std::ffi::OsStr;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use hashbrown::HashSet;
use image::{GenericImage, GenericImageView};
use indexmap::IndexSet;
use tiled::{Properties, PropertyValue, TileLayer};
use toml_edit::{value, Array, Item, Table, Value};
use tiledpd::AddDependencies;
// use tiledpd::properties::PropertyValue2;
use tiledpd::properties::{ArchivedPropertyValue, PropertyValue as PVPD};
use tiledpd::rkyv::util::AlignedVec;
use tiledpd::tilemap::{ArchivedTilemap, Tile, Tilemap};
use tiledpd::tileset::ArchivedTileset;
use crate::pdtiled::{convert_map, convert_property, convert_tileset};

fn main() {    
    let game_toml = std::fs::read_to_string("game/Cargo.toml").unwrap();
    let mut game_toml = toml_edit::DocumentMut::from_str(&game_toml).unwrap();
    let playdate = &mut game_toml["package"]["metadata"]["playdate"];
    // increment build number
    {
        println!("incrementing build number");
        let build_number = &mut playdate["build-number"];
        *build_number = value(build_number.as_integer().unwrap() + 1);
    }
    // process assets
    {
        println!("processing assets");
        let mut asset_table = Table::new();
        
        let assets = run_assets();
        for asset in assets {
            let asset = asset.to_string_lossy().to_string();
            let destination = format!("assets/{}", asset);
            let source = format!("../{}/{}/{}", ASSET_PATH, EXPORT_FOLDER, asset);
            asset_table.insert(&destination, source.into());
        }

        playdate["assets"] = Item::Table(asset_table);
    }
    
    
    std::fs::write("game/Cargo.toml", game_toml.to_string()).unwrap();
    
    // run_game(true)
}

pub fn run_game(device: bool) {
    let target = if device { "--device" } else { "--simulator" };

    Command::new("cargo")
        .args(["playdate", "run", "-p", "game", target])
        .arg("--release")
        .spawn().unwrap()
        .wait().unwrap();
}

pub fn run_assets() -> Vec<PathBuf> {
    let manifest = std::fs::read_to_string("manifest.toml").unwrap();
    let manifest = toml_edit::DocumentMut::from_str(&manifest).unwrap();
    let err = std::fs::remove_dir_all(Path::new(ASSET_PATH).join(EXPORT_FOLDER));
    if let Err(err) = err {
        if err.kind() != ErrorKind::NotFound {
            panic!("{}", err);
        }
    }

    let mut assets = Assets::default();

    for asset in manifest["assets"].as_array().unwrap() {
        let s = asset.as_str().unwrap().to_string();
        let path = path::pd_to_pc(s);
        assets.add_asset(path);
    }
    
    while let Some(asset) = assets.fulfill_next() {
        println!("↳{:?}", asset);
        let extension = asset.extension();
        if extension == Some(OsStr::new("tmx")) || extension == Some(OsStr::new("tmb")) {
            process_map(&asset, &mut assets);
        } else if extension == Some(OsStr::new("tsx")) || extension == Some(OsStr::new("tsb")) {
            process_tileset(&asset, &mut assets);
        } else {
            process_default(&asset);
        }
    }
    
    assets.finish()
}

#[derive(Default)]
struct Assets {
    processed_assets: IndexSet<PathBuf>,
    assets_to_process: IndexSet<PathBuf>,
}

impl Assets {
    /// Path should be **Relative to `assets` folder**.
    pub fn add_asset(&mut self, mut asset: PathBuf) {
        // if asset.extension() == Some(OsStr::new("tmx")) {
        //     asset.set_extension(OsStr::new("tmb"));
        // }
        // if asset.extension() == Some(OsStr::new("tsx")) {
        //     asset.set_extension(OsStr::new("tsb"));
        // }
        
        if self.processed_assets.contains(&asset) {
            return;
        }
        
        self.assets_to_process.insert(asset);
    }
    
    pub fn add_pd_asset(&mut self, asset: String) {
        
    }
    
    pub fn fulfill_next(&mut self) -> Option<PathBuf> {
        let path = self.assets_to_process.pop()?;
        
        // dbg!(&path);
        
        let (i, b) = self.processed_assets.insert_full(path);
        assert!(b);
        
        self.processed_assets.get_index(i).cloned()
    }
    
    pub fn finish(self) -> Vec<PathBuf> {
        self.processed_assets.into_iter()
            .collect()
    }
}

const ASSET_PATH: &str = "assets";
const EXPORT_FOLDER: &str = "export";
fn process_map(path: &Path, assets: &mut Assets) {
    let true_map_path = Path::new(ASSET_PATH).join(path);
    
    let map = tiled::Loader::new().load_tmx_map(&true_map_path).unwrap();
    let map = convert_map(map);
    
    let bytes = tiledpd::rkyv::to_bytes::<tiledpd::RkyvError>(&map).unwrap();
    
    let archived_map = tiledpd::rkyv::access::<ArchivedTilemap, tiledpd::RkyvError>(&bytes).unwrap();
    dbg!(archived_map);
    let mut asset_buf = HashSet::new();
    archived_map.add_dependencies(&mut asset_buf);
    for asset in asset_buf {
        let asset = asset.trim_start_matches("assets\\");
        let path = Path::new(asset);
        let path = true_map_path.parent().unwrap().join(path);
        let path = path.strip_prefix("assets\\").unwrap().to_owned();
        
        assets.add_asset(path);
    }
    
    let bytes = lz4_flex::compress_prepend_size(&bytes);
    
    let export_path = Path::new(ASSET_PATH).join(EXPORT_FOLDER).join(path);
    
    if let Some(parent) = export_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(export_path, &bytes).unwrap();
}

fn process_tileset(path: &Path, assets: &mut Assets) {
    let true_set_path = Path::new(ASSET_PATH).join(path);
    let tileset = tiled::Loader::new().load_tsx_tileset(&true_set_path).unwrap();
    let tileset = convert_tileset(tileset);
    
    let bytes = tiledpd::rkyv::to_bytes::<tiledpd::RkyvError>(&tileset).unwrap();
    
    let archived_set = tiledpd::rkyv::access::<ArchivedTileset, tiledpd::RkyvError>(&bytes).unwrap();
    let mut asset_buf = HashSet::new();
    archived_set.add_dependencies(&mut asset_buf);
    for asset in asset_buf {
        let asset = asset.trim_start_matches("assets\\");
        let path = Path::new(asset);
        let path = true_set_path.parent().unwrap().join(path);
        let path = path.strip_prefix("assets\\").unwrap().to_owned();

        assets.add_asset(path);
    }

    let bytes = lz4_flex::compress_prepend_size(&bytes);

    let export_path = Path::new(ASSET_PATH).join(EXPORT_FOLDER).join(path);

    if let Some(parent) = export_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(export_path, &bytes).unwrap();
}

pub mod path {
    use std::ffi::OsStr;
    use std::path::{PathBuf};

    pub fn pc_to_pd(mut path_pc: PathBuf) -> String {
        match path_pc.extension() {
            None => {}
            Some(x) if x == OsStr::new("tmx") => {
                path_pc.set_extension("tmb");
            }
            Some(x) if x == OsStr::new("tsx") => {
                path_pc.set_extension("tsb");
            },
            Some(x) if x == OsStr::new("png") => {
                path_pc.set_extension("pdi");
            },
            Some(_) => {},
        }
        
        let mut path = path_pc.to_string_lossy().to_string();
        path = path.replace("\\", "/");
        
        path
    }
    
    pub fn pd_to_pc(path: String) -> PathBuf {
        let mut path = PathBuf::from(path);
        match path.extension() {
            None => {},
            Some(x) if x == OsStr::new("tmb") => {
                path.set_extension("tmx");
            }
            Some(x) if x == OsStr::new("tsb") => {
                path.set_extension("tsx");
            },
            Some(x) if x == OsStr::new("pdi") => {
                path.set_extension("png");
            },
            Some(_) => {}
        }
        
        path
    }
}

/// Copies file to export folder. Path must be relative to `assets` folder.
/// I.e. `"tiles.png"` corresponds to `"./assets/tiles.png"`
pub fn process_default(path: &Path) {
    let old_path = Path::new(ASSET_PATH).join(path);
    let new_path = Path::new(ASSET_PATH).join(EXPORT_FOLDER).join(path);
    // dbg!(&old_path, &new_path);
    
    std::fs::create_dir_all(new_path.parent().unwrap()).unwrap();
    std::fs::copy(old_path, new_path).unwrap();
    
    // path.parent().unwrap()
    // std::fs::create_dir_all(path.parent())
}

struct AssetPath {
    
}

impl AssetPath {
    // name of the file to load in-game
    pub fn pd_ref(&self) -> String {
        todo!()
    }
    
    // name of the file on pc
    pub fn pc_path(&self) -> PathBuf {
        todo!()
    }
    
    // name of the file to put in the game toml=r
    pub fn asset_ref(&self) -> String {
        todo!()
    }
}

