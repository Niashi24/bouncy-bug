use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

#[derive(Clone, Eq, PartialEq, Hash, Debug, Serialize, Deserialize)]
pub(crate) struct TypeExport {
    pub id: u32,
    pub name: String,
    #[serde(flatten)]
    pub type_data: TypeData,
}

#[derive(Clone, Eq, PartialEq, Hash, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub(crate) enum TypeData {
    Enum(Enum),
    Class(Class),
}

#[derive(Clone, Eq, PartialEq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Class {
    pub use_as: Vec<UseAs>,
    pub color: String,
    pub draw_fill: bool,
    pub members: Vec<Member>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Member {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property_type: Option<String>,
    #[serde(rename = "type")]
    pub type_field: FieldType,
    pub value: serde_json::Value,
}

#[derive(Clone, Eq, PartialEq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Enum {
    pub storage_type: StorageType,
    pub values: Vec<String>,
    pub values_as_flags: bool,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum StorageType {
    String,
    Int,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum FieldType {
    Bool,
    Color,
    Float,
    File,
    Int,
    Object,
    String,
    Class,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum UseAs {
    Property,
    Map,
    Layer,
    Object,
    Tile,
    Tileset,
    WangColor,
    WangSet,
    Project,
}
