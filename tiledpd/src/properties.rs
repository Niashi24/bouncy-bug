use alloc::string::String;
use core::fmt::Debug;
use hashbrown::HashMap;
use rkyv::{Archive, Deserialize, Serialize};

/// Represents a custom property's value.
///
/// Also read the [TMX docs](https://doc.mapeditor.org/en/stable/reference/tmx-map-format/#tmx-properties).
#[derive(Debug, PartialEq, Clone, Archive, Deserialize, Serialize)]
#[rkyv(serialize_bounds(__S: rkyv::ser::Writer + rkyv::ser::Allocator, __S::Error: rkyv::rancor::Source))]
#[rkyv(deserialize_bounds())]
#[rkyv(bytecheck(bounds(__C: rkyv::validation::ArchiveContext)))]
#[rkyv(derive(Debug))]
pub enum PropertyValue {
    /// A boolean value. Corresponds to the `bool` property type.
    BoolValue(bool),
    /// A floating point value. Corresponds to the `float` property type.
    FloatValue(f32),
    /// A signed integer value. Corresponds to the `int` property type.
    IntValue(i32),
    // /// A color value. Corresponds to the `color` property type.
    // ColorValue(Color),
    /// A string value. Corresponds to the `string` property type.
    StringValue(String),
    /// A filepath value. Corresponds to the `file` property type.
    /// Holds the path relative to the map or tileset.
    FileValue(String),
    /// An object ID value. Corresponds to the `object` property type.
    /// Holds the id of a referenced object, or 0 if unset.
    ObjectValue(u32),
    /// A class value. Corresponds to the `class` property type.
    /// Holds the type name and a set of properties.
    ClassValue {
        /// The type name.
        property_type: String,
        /// A set of properties.
        #[rkyv(omit_bounds)]
        properties: Properties,
    },
}

// impl Debug for ArchivedPropertyValue {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         match self {
//             ArchivedPropertyValue::BoolValue(b) => f.debug_tuple("ArchivedPropertyValue")
//                 .field(b)
//                 .finish(),
//             ArchivedPropertyValue::FloatValue(_) => {}
//             ArchivedPropertyValue::IntValue(_) => {}
//             ArchivedPropertyValue::StringValue(_) => {}
//             ArchivedPropertyValue::FileValue(_) => {}
//             ArchivedPropertyValue::ObjectValue(_) => {}
//             ArchivedPropertyValue::ClassValue { .. } => {}
//         }
//     }
// }


/// A custom property container.
pub type Properties = HashMap<String, PropertyValue>;

#[cfg(test)]
mod test {
    use rkyv::access;
    use rkyv::rancor::Error;
    use crate::properties::{ArchivedPropertyValue, PropertyValue};

    #[test]
    pub fn test_serialize() {
        let value = PropertyValue::IntValue(5);
        let buf = rkyv::to_bytes::<Error>(&value).unwrap();
        let deserialized = access::<ArchivedPropertyValue, Error>(&buf).unwrap();
        match deserialized {
            ArchivedPropertyValue::IntValue(x) => assert_eq!(x.to_native(), 5),
            _ => panic!(),
        }
    }
}
