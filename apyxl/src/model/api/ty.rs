use std::fmt::Debug;

use crate::model::entity::{EntityMut, FindEntity};
use anyhow::Result;

use crate::model::{Entity, EntityId};

/// A type within the language or API. Types other than [TypeRef::Api] are assumed to always
/// exist during API validation and can be used by [crate::Generator]s to map to the relevant known
/// type in the target language without additional setup.
///
/// Arbitrary user-defined types can be added with [TypeRef::User].
///
/// Types within the parsed API will have the [TypeRef::Api] type, and validation will ensure they
/// exist after the API is built.
///
/// This is generic so that view::Type can provide relevant view types for variants with data.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum BaseType<TypeRef, ApiType, UserTypeName>
where
    TypeRef: Debug + Clone,
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
    USIZE,

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

    /// An array of the contained type.
    Array(Box<TypeRef>),

    /// A key-value map.
    Map {
        key: Box<TypeRef>,
        value: Box<TypeRef>,
    },

    /// An optional type, i.e. a type that also includes whether it is set or not.
    /// Sometimes called a nullable type.
    Optional(Box<TypeRef>),
}
pub type UserTypeName = String;
pub type Type = BaseType<TypeRef, EntityId, UserTypeName>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TypeRef {
    pub value: Type,
    pub semantics: Semantics,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Semantics {
    Value,
    Ref,
    Mut,
}

impl TypeRef {
    pub fn new(value: Type, semantics: Semantics) -> Self {
        Self { value, semantics }
    }

    pub fn new_api(value: &str, semantics: Semantics) -> Result<Self> {
        Ok(Self {
            value: Type::new_api(value)?,
            semantics,
        })
    }

    pub fn new_array(ty: TypeRef, semantics: Semantics) -> Self {
        Self::new(Type::Array(Box::new(ty)), semantics)
    }

    pub fn new_map(key_ty: TypeRef, value_ty: TypeRef, semantics: Semantics) -> Self {
        Self::new(
            Type::Map {
                key: Box::new(key_ty),
                value: Box::new(value_ty),
            },
            semantics,
        )
    }

    pub fn new_optional(ty: TypeRef, semantics: Semantics) -> Self {
        Self::new(Type::Optional(Box::new(ty)), semantics)
    }

    pub fn is_primitive(&self) -> bool {
        self.value.is_primitive()
    }
}

impl<T: Debug + Clone, E: Debug + Clone, U: Debug + Clone> BaseType<T, E, U> {
    pub fn is_primitive(&self) -> bool {
        match self {
            BaseType::Bool
            | BaseType::U8
            | BaseType::U16
            | BaseType::U32
            | BaseType::U64
            | BaseType::U128
            | BaseType::USIZE
            | BaseType::I8
            | BaseType::I16
            | BaseType::I32
            | BaseType::I64
            | BaseType::I128
            | BaseType::F8
            | BaseType::F16
            | BaseType::F32
            | BaseType::F64
            | BaseType::F128 => true,

            BaseType::String
            | BaseType::Bytes
            | BaseType::User(_)
            | BaseType::Api(_)
            | BaseType::Array(_)
            | BaseType::Map { .. }
            | BaseType::Optional(_) => false,
        }
    }
}

impl Type {
    pub fn new_api(value: &str) -> Result<Self> {
        Ok(Self::Api(EntityId::try_from(value)?))
    }

    pub fn api(&self) -> Option<&EntityId> {
        if let Self::Api(id) = &self {
            Some(id)
        } else {
            None
        }
    }

    pub fn new_array(ty: TypeRef) -> Self {
        Self::Array(Box::new(ty))
    }

    pub fn new_map(key_ty: TypeRef, value_ty: TypeRef) -> Self {
        Self::Map {
            key: Box::new(key_ty),
            value: Box::new(value_ty),
        }
    }

    pub fn new_optional(ty: TypeRef) -> Self {
        Self::Optional(Box::new(ty))
    }
}

impl<'api> FindEntity<'api> for TypeRef {
    fn find_entity<'a>(&'a self, id: EntityId) -> Option<Entity<'a, 'api>> {
        if id.is_empty() {
            Some(Entity::Type(self))
        } else {
            None
        }
    }

    fn find_entity_mut<'a>(&'a mut self, id: EntityId) -> Option<EntityMut<'a, 'api>> {
        if id.is_empty() {
            Some(EntityMut::Type(self))
        } else {
            None
        }
    }
}
