use alloc::string::String;
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Serialize, Deserialize, Debug)]
#[rkyv(derive(Debug))]
pub struct Gif {
    pub image_path: String,
    pub fps: f32,
}