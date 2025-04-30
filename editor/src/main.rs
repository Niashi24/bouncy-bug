mod pdtiled;

use std::cell::LazyCell;
use std::ffi::OsStr;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use std::sync::LazyLock;
use hashbrown::HashSet;
use image::{GenericImage, GenericImageView};
use indexmap::IndexSet;
use regex::Regex;
use tiled::{Properties, PropertyValue, TileLayer};
use toml_edit::{value, Array, Item, Table, Value};
use tiledpd::AddDependencies;
use tiledpd::dependencies::AddDependenciesMut;
// use tiledpd::properties::PropertyValue2;
use tiledpd::properties::{ArchivedPropertyValue, PropertyValue as PVPD};
use tiledpd::rkyv::util::AlignedVec;
use tiledpd::tilemap::{ArchivedTilemap, Tile, Tilemap};
use tiledpd::tileset::ArchivedTileset;
use crate::pdtiled::{convert_map, convert_property, convert_tileset};

fn main() -> anyhow::Result<()> {
    let game_toml = std::fs::read_to_string("game/Cargo.toml")?;
    let mut game_toml = toml_edit::DocumentMut::from_str(&game_toml)?;
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
        
        let _ = fs::remove_dir_all("assets\\export");
        
        let _ = run_assets();
        
        // get all files in export folder (recursively)
        let mut files = Vec::new();
        let mut directories = vec![PathBuf::from("assets\\export")];
        while let Some(directory) = directories.pop() {
            for file in fs::read_dir(&directory)? {
                let file = file?;
                let metadata = file.metadata()?;
                if metadata.is_dir() {
                    directories.push(file.path());
                } else if metadata.is_file() {
                    files.push(file.path());
                }
            }
        }
        
        for file in files {
            let file = file.to_string_lossy().replace("\\", "/");
            let source = format!("../{}", file);
            let destination = file.replacen("/export", "", 1);
            asset_table.insert(&destination, source.into());
        }
        
        // fs::read_di
        // for asset in assets {
        //     let asset = asset.to_string_lossy().to_string();
        //     let destination = format!("assets\\{}", asset);
        //     let source = format!("../{}/{}/{}", ASSET_PATH, EXPORT_FOLDER, asset);
        //     asset_table.insert(&destination, source.into());
        // }

        playdate["assets"] = Item::Table(asset_table);
    }
    
    
    std::fs::write("game/Cargo.toml", game_toml.to_string())?;
    
    Ok(())
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
        assets.add_asset(path, true);
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
    /// Path should be **Relative to `assets` folder**, representing
    /// the path of a (possibly unprocessed) asset. Use `process = true` to skip the processing of
    /// an asset if you know the asset is already processed (saving directly to export folder).
    /// Ex. "folderA/tilemap.tmx" but not "folderA/tilemap.tmb".
    pub fn add_asset(&mut self, asset: PathBuf, process: bool) {
        
        if !process {
            self.processed_assets.insert(asset);
            return;
        }
        
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
    println!("processing tilemap: {:?}", path);
    
    let true_map_path = Path::new(ASSET_PATH).join(path);
    
    let map = tiled::Loader::new().load_tmx_map(&true_map_path).unwrap();
    let mut map = convert_map(map);


    let mut asset_paths = Vec::new();
    map.add_dependencies_mut(&mut asset_paths);

    process_asset_paths(assets, asset_paths, &true_map_path);
    
    let bytes = tiledpd::rkyv::to_bytes::<tiledpd::RkyvError>(&map).unwrap();
    // dbg!(tiledpd::rkyv::access::<ArchivedTilemap, tiledpd::RkyvError>(&bytes).unwrap());
    
    let bytes = lz4_flex::compress_prepend_size(&bytes);

    let mut path = path.to_path_buf();
    path.set_extension("tmb");
    
    let export_path = Path::new(ASSET_PATH).join(EXPORT_FOLDER).join(path);
    
    if let Some(parent) = export_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(export_path, &bytes).unwrap();
}

fn process_tileset(path: &Path, assets: &mut Assets) {
    println!("processing tileset: {:?}", path);
    
    let true_set_path = Path::new(ASSET_PATH).join(path);
    let tileset = tiled::Loader::new().load_tsx_tileset(&true_set_path).unwrap();
    let mut tileset = convert_tileset(tileset);
    
    let mut asset_paths = Vec::new();
    tileset.add_dependencies_mut(&mut asset_paths);
    
    process_asset_paths(assets, asset_paths, &true_set_path);
    
    let bytes = tiledpd::rkyv::to_bytes::<tiledpd::RkyvError>(&tileset).unwrap();
    // dbg!(tiledpd::rkyv::access::<ArchivedTileset, tiledpd::RkyvError>(&bytes).unwrap());

    let bytes = lz4_flex::compress_prepend_size(&bytes);
    
    let mut path = path.to_path_buf();
    path.set_extension("tsb");

    let export_path = Path::new(ASSET_PATH).join(EXPORT_FOLDER).join(path);

    if let Some(parent) = export_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(export_path, &bytes).unwrap();
}


fn process_asset_paths(assets: &mut Assets, asset_paths: Vec<&mut String>, origin: &Path) {
    for asset in asset_paths {

        const EXTENSIONS: &[[&str; 3]] = &[
            ["tmx", "tmb", "tmb"],
            ["tsx", "tsb", "tsb"],
            ["png", "png", "pdi"],
        ];
        
        static IMAGE_TABLE_REGEX: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r#"(?<name>.*)-table-\d+(?:-\d+)?\..+"#).unwrap());
        
        
        
        
        if let Some(captures) = IMAGE_TABLE_REGEX.captures(&asset) {
            // pc stuff
            {
                let path = Path::new(asset.trim_start_matches("assets\\"));
                let mut path = origin.parent().unwrap().join(path);
                path = path.strip_prefix("assets\\").unwrap().to_path_buf();

                assets.add_asset(path, true);
            }
            // playdate stuff
            {
                // start with "tiles-16-16.png"
                // now "tiles"
                let name = &captures["name"];
                dbg!(name);
                // now "tiles" (not sure why this is here, but we ball)
                let path = Path::new(name.trim_start_matches("assets\\"));
                // now "parent\tiles"
                let mut path = origin.parent().unwrap().join(path);
                // now "parent/tiles"
                *asset = path.to_string_lossy().to_string().replace("\\", "/");
            }
            // correct playdate file name, now let's add it to assets
            
        } else {
            let path = Path::new(asset.trim_start_matches("assets\\"));
            let mut path = origin.parent().unwrap().join(path);
            
            let extension = EXTENSIONS.iter()
                .find(|[x, _, _]| asset.ends_with(x))
                .copied()
                .unwrap_or_else(|| {
                    dbg!(&asset);
                    let i = asset.rfind(".").expect("get extension of asset");
                    let extension = asset.split_at(i + 1).1;
                    [extension; 3]
                });

            let [pc, _export, pd] = extension;

            path.set_extension(pc);

            let mut game = path.clone();
            game.set_extension(pd);
            let game = game.to_string_lossy().to_string().replace("\\", "/");
            *asset = game;
            
            path = path.strip_prefix("assets\\").unwrap().to_path_buf();

            assets.add_asset(path, true);
        }
    }
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
/// I.e. `"tiles.png"` corresponds to `"./assets\\tiles.png"`
pub fn process_default(path: &Path) {
    let old_path = Path::new(ASSET_PATH).join(path);
    let new_path = Path::new(ASSET_PATH).join(EXPORT_FOLDER).join(path);
    // dbg!(&old_path, &new_path);
    
    std::fs::create_dir_all(new_path.parent().unwrap()).unwrap();
    std::fs::copy(old_path, new_path).unwrap();
    
    // path.parent().unwrap()
    // std::fs::create_dir_all(path.parent())
}
