use std::fmt::Debug;

use crate::model::entity::{EntityMut, FindEntity};
use crate::model::{Entity, EntityId, Namespace};
use anyhow::{anyhow, Result};
use itertools::Itertools;

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
    StringView,

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

    /// A function type, typically used to represent events, but could also be used for callable
    /// predicates/functors in some niche situations.
    Function {
        params: Vec<Box<TypeRef>>,
        return_ty: Option<Box<TypeRef>>,
    },
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

    pub fn new_function(
        params: impl IntoIterator<Item = TypeRef>,
        return_ty: Option<TypeRef>,
        semantics: Semantics,
    ) -> Self {
        Self::new(
            Type::Function {
                params: params.into_iter().map(Box::new).collect_vec(),
                return_ty: return_ty.map(Box::new),
            },
            semantics,
        )
    }

    pub fn is_primitive(&self, api: &Namespace) -> bool {
        self.value.is_primitive(api)
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

    pub fn new_function(
        params: impl IntoIterator<Item = TypeRef>,
        return_ty: Option<TypeRef>,
    ) -> Self {
        Self::Function {
            params: params.into_iter().map(Box::new).collect_vec(),
            return_ty: return_ty.map(Box::new),
        }
    }

    pub fn is_primitive(&self, api: &Namespace) -> bool {
        match self {
            Type::Api(api_ty) => {
                if let Some(ty) = api.find_ty_alias(api_ty) {
                    ty.target_ty.is_primitive(api)
                } else {
                    false
                }
            }

            Type::Bool
            | Type::U8
            | Type::U16
            | Type::U32
            | Type::U64
            | Type::U128
            | Type::USIZE
            | Type::I8
            | Type::I16
            | Type::I32
            | Type::I64
            | Type::I128
            | Type::F8
            | Type::F16
            | Type::F32
            | Type::F64
            | Type::F128
            | Type::StringView => true,

            Type::String
            | Type::Bytes
            | Type::User(_)
            | Type::Array(_)
            | Type::Map { .. }
            | Type::Optional(_)
            | Type::Function { .. } => false,
        }
    }
}

impl<'api> FindEntity<'api> for TypeRef {
    fn qualify_id(&self, mut id: EntityId, referenceable: bool) -> Result<EntityId> {
        if referenceable {
            return Err(anyhow!("types are not referenceable"));
        }
        match id.pop_front() {
            Some((_, name)) => Err(anyhow!(
                "failed to qualify_id {}.{} - ty has no children",
                name,
                id
            )),
            None => Ok(EntityId::default()),
        }
    }

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
