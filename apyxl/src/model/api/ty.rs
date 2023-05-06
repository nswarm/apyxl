use crate::model::EntityId;
use std::fmt::Debug;

/// A type within the language or API. Types other than [Type::Api] are assumed to always
/// exist during API validation and can be used by [crate::Generator]s to map to the relevant known
/// type in the target language without additional setup.
///
/// Arbitrary user-defined types can be added with [Type::User].
///
/// Types within the parsed API will have the [Type::Api] type, and validation will ensure they
/// exist after the API is built.
///
/// This is generic so that view::Type can provide relevant view types for variants with data.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum BaseType<ApiType, UserTypeName>
where
    ApiType: Debug + Clone,
    UserTypeName: Debug + Clone,
{
    Bool,

    // Unsigned integers.
    U8,
    U16,
    U32,
    U64,
    U128,

    // Signed integers.
    I8,
    I16,
    I32,
    I64,
    I128,

    // Floating point numbers.
    F8,
    F16,
    F32,
    F64,
    F128,

    // Strings.
    String,

    // Arbitrary sequence of bytes.
    Bytes,

    /// This can be useful when there is a type that is not within the parsing set, but a
    /// user [crate::Generator]'s target language has support for that type.
    ///
    /// Example: You have a type in the source language called `UUID` which is not within the file
    /// set you parse. You can add `Type::User("uuid")` as. Now any [crate::Generator] you
    /// write can check the for the name `uuid` and map that to its target language equivalent.
    User(UserTypeName),

    /// Reference to another type within the API. This must reference an existing type within
    /// the API when built.
    Api(ApiType),
}
pub type UserTypeName = String;
pub type Type = BaseType<EntityId, UserTypeName>;

impl Type {
    pub fn new_api(value: &str) -> Self {
        Self::Api(EntityId::from(value))
    }

    pub fn api(&self) -> Option<&EntityId> {
        if let Type::Api(id) = self {
            Some(id)
        } else {
            None
        }
    }
}