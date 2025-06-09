use std::fmt::Debug;

use crate::model::entity::{AsEntity, EntityMut, FindEntity, FindEntityMut};
use anyhow::Result;

use crate::model::{Entity, EntityId, Namespace};

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
            | Type::Optional(_) => false,
        }
    }
}

impl<'api> AsEntity<'api> for TypeRef {
    fn as_entity<'a>(&'a self) -> Entity<'a, 'api> {
        Entity::Type(self)
    }

    fn as_entity_mut<'a>(&'a mut self) -> EntityMut<'a, 'api> {
        EntityMut::Type(self)
    }
}

impl<'api> FindEntity<'api> for TypeRef {
    fn find_entity<'a, F>(
        &'a self,
        id: EntityId,
        predicate: F,
    ) -> Option<(EntityId, Entity<'a, 'api>)>
    where
        'a: 'api,
        F: Fn(&EntityId, &Entity<'a, 'api>) -> bool,
    {
        if predicate(&id, &self.as_entity()) {
            Some((id, self.as_entity()))
        } else {
            None
        }
    }

    fn find_entity_by_id<'a>(&'a self, id: EntityId) -> Option<Entity<'a, 'api>> {
        if id.is_empty() {
            Some(Entity::Type(self))
        } else {
            None
        }
    }

    fn collect_entities<'a, F>(
        &'a self,
        id: EntityId,
        results: &mut Vec<(EntityId, Entity<'a, 'api>)>,
        predicate: F,
    ) where
        'a: 'api,
        F: Fn(&EntityId, &Entity<'a, 'api>) -> bool,
    {
        if predicate(&id, &self.as_entity()) {
            results.push((id, self.as_entity()))
        }
    }

    fn collect_types<'a>(&'a self, id: EntityId, results: &mut Vec<(EntityId, &'a TypeRef)>) {
        results.push((id, self))
    }
}

impl<'api> FindEntityMut<'api> for TypeRef {
    fn find_entity_mut<'a, F>(
        &'a mut self,
        id: EntityId,
        predicate: F,
    ) -> Option<(EntityId, EntityMut<'a, 'api>)>
    where
        'a: 'api,
        F: for<'p> Fn(&'p EntityId, &'p EntityMut<'a, 'api>) -> bool,
    {
        todo!()
    }

    fn find_entity_by_id_mut<'a>(&'a mut self, id: EntityId) -> Option<EntityMut<'a, 'api>> {
        if id.is_empty() {
            Some(EntityMut::Type(self))
        } else {
            None
        }
    }

    fn collect_entities_mut<'a, F>(
        &'a mut self,
        id: EntityId,
        results: &mut Vec<(EntityId, EntityMut<'a, 'api>)>,
        predicate: F,
    ) where
        'a: 'api,
        F: Fn(&EntityId, &EntityMut<'a, 'api>) -> bool,
    {
        todo!()
    }

    fn collect_types_mut<'a>(
        &'a mut self,
        id: EntityId,
        results: &mut Vec<(EntityId, &'a mut TypeRef)>,
    ) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    mod find_entity {
        use crate::model::entity::test_macros::{
            find_entity_match_predicate, find_entity_no_match_predicate,
        };
        use crate::model::entity::FindEntity;
        use crate::model::{Entity, EntityId, EntityType, Semantics, Type, TypeRef};

        #[test]
        fn match_predicate() {
            let ty = TypeRef::new(Type::U32, Semantics::Value);
            find_entity_match_predicate!(ty, Type);
        }

        #[test]
        fn no_match_predicate() {
            let ty = TypeRef::new(Type::U32, Semantics::Value);
            find_entity_no_match_predicate!(ty, Dto);
        }
    }

    mod find_entity_by_id {
        use crate::model::entity::test_macros::{
            find_entity_by_id_found, find_entity_by_id_not_found,
        };
        use crate::model::entity::FindEntity;
        use crate::model::{Entity, EntityId, EntityType, Semantics, Type, TypeRef};

        #[test]
        fn found() {
            let ty = TypeRef::new(Type::U32, Semantics::Value);
            find_entity_by_id_found!(ty, Type);
        }

        #[test]
        fn not_found() {
            let ty = TypeRef::new(Type::U32, Semantics::Value);
            find_entity_by_id_not_found!(ty);
        }
    }

    mod collect_entities {
        use crate::model::entity::test_macros::{
            collect_entities_match_predicate, collect_entities_no_match_predicate,
        };
        use crate::model::entity::FindEntity;
        use crate::model::{Entity, EntityId, EntityType, Semantics, Type, TypeRef};

        #[test]
        fn match_predicate() {
            let ty = TypeRef::new(Type::U32, Semantics::Value);
            collect_entities_match_predicate!(ty, Type);
        }

        #[test]
        fn no_match_predicate() {
            let ty = TypeRef::new(Type::U32, Semantics::Value);
            collect_entities_no_match_predicate!(ty, Dto);
        }
    }

    mod collect_types {
        use crate::model::entity::FindEntity;
        use crate::model::{EntityId, Semantics, Type, TypeRef};

        #[test]
        fn found() {
            let ty = TypeRef::new(Type::U32, Semantics::Value);
            let expected_id = EntityId::new_unqualified("something");
            let mut collected = Vec::new();

            ty.collect_types(expected_id.clone(), &mut collected);
            assert!(!collected.is_empty());

            let (id, actual) = collected.first().unwrap();
            assert_eq!(id, &expected_id);
            assert_eq!(*actual, &ty);
        }

        // collect types will always return something if it reaches a type!
    }
}
