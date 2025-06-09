use crate::model::{Dto, EntityId, Enum, Field, Namespace, NamespaceChild, Rpc, TypeAlias, TypeRef};
use anyhow::anyhow;
use std::collections::VecDeque;
use std::iter::Map;
use std::marker::PhantomData;
use std::slice::IterMut;
use std::vec::IntoIter;

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Copy, Clone)]
pub enum EntityType {
    None, // Unqualified EntityIds.
    Namespace,
    Dto,
    Rpc,
    Enum,
    Field,
    TypeAlias,
    Type,
}

/// Reference to a specific entity within an API.
#[derive(Debug, Eq, PartialEq)]
pub enum Entity<'a, 'api> {
    Namespace(&'a Namespace<'api>),
    Dto(&'a Dto<'api>),
    Rpc(&'a Rpc<'api>),
    Enum(&'a Enum<'api>),
    Field(&'a Field<'api>),
    TypeAlias(&'a TypeAlias<'api>),
    Type(&'a TypeRef),
}

/// Mutable reference to a specific entity within an API.
#[derive(Debug, Eq, PartialEq)]
pub enum EntityMut<'a, 'api> {
    Namespace(&'a mut Namespace<'api>),
    Dto(&'a mut Dto<'api>),
    Rpc(&'a mut Rpc<'api>),
    Enum(&'a mut Enum<'api>),
    Field(&'a mut Field<'api>),
    TypeAlias(&'a mut TypeAlias<'api>),
    Type(&'a mut TypeRef),
}

/// Holds an immutable reference to a single entity in an API. Supports searching and collecting
/// entities in the entire hierarchy with the support of the [FindEntity] trait.
impl<'a, 'api> Entity<'a, 'api>
where
    'a: 'api,
{
    pub fn ty(&self) -> EntityType {
        match self {
            Entity::Namespace(_) => EntityType::Namespace,
            Entity::Dto(_) => EntityType::Dto,
            Entity::Rpc(_) => EntityType::Rpc,
            Entity::Enum(_) => EntityType::Enum,
            Entity::Field(_) => EntityType::Field,
            Entity::Type(_) => EntityType::Type,
            Entity::TypeAlias(_) => EntityType::TypeAlias,
        }
    }

    fn find_entity<F>(self, id: EntityId, predicate: F) -> Option<(EntityId, Entity<'a, 'api>)>
    where
        F: Fn(&EntityId, &Entity<'a, 'api>) -> bool,
    {
        match self {
            Entity::Namespace(e) => e.find_entity(id, predicate),
            Entity::Dto(e) => e.find_entity(id, predicate),
            Entity::Rpc(e) => e.find_entity(id, predicate),
            Entity::Enum(e) => e.find_entity(id, predicate),
            Entity::Field(e) => e.find_entity(id, predicate),
            Entity::TypeAlias(e) => e.find_entity(id, predicate),
            Entity::Type(e) => e.find_entity(id, predicate),
        }
    }

    fn find_entity_by_id(self, id: EntityId) -> Option<Entity<'a, 'api>> {
        match self {
            Entity::Namespace(e) => e.find_entity_by_id(id),
            Entity::Dto(e) => e.find_entity_by_id(id),
            Entity::Rpc(e) => e.find_entity_by_id(id),
            Entity::Enum(e) => e.find_entity_by_id(id),
            Entity::Field(e) => e.find_entity_by_id(id),
            Entity::TypeAlias(e) => e.find_entity_by_id(id),
            Entity::Type(e) => e.find_entity_by_id(id),
        }
    }

    fn collect_entities<F>(
        self,
        id: EntityId,
        results: &mut Vec<(EntityId, Entity<'a, 'api>)>,
        predicate: F,
    ) where
        F: Fn(&EntityId, &Entity<'a, 'api>) -> bool,
    {
        match self {
            Entity::Namespace(e) => e.collect_entities(id, results, predicate),
            Entity::Dto(e) => e.collect_entities(id, results, predicate),
            Entity::Rpc(e) => e.collect_entities(id, results, predicate),
            Entity::Enum(e) => e.collect_entities(id, results, predicate),
            Entity::Field(e) => e.collect_entities(id, results, predicate),
            Entity::TypeAlias(e) => e.collect_entities(id, results, predicate),
            Entity::Type(e) => e.collect_entities(id, results, predicate),
        }
    }

    fn collect_types(self, id: EntityId, results: &mut Vec<(EntityId, &'a TypeRef)>) {
        match self {
            Entity::Namespace(e) => e.collect_types(id, results),
            Entity::Dto(e) => e.collect_types(id, results),
            Entity::Rpc(e) => e.collect_types(id, results),
            Entity::Enum(e) => e.collect_types(id, results),
            Entity::Field(e) => e.collect_types(id, results),
            Entity::TypeAlias(e) => e.collect_types(id, results),
            Entity::Type(e) => e.collect_types(id, results),
        }
    }
}

/// Holds an immutable reference to a single entity in an API. Supports searching and collecting
/// entities in the entire hierarchy with the support of the [FindEntity] and [FindEntityMut] traits.
impl<'a, 'api> EntityMut<'a, 'api>
where
    'a: 'api,
{
    fn into_immutable(self) -> Entity<'a, 'api> {
        match self {
            EntityMut::Namespace(e) => Entity::Namespace(e),
            EntityMut::Dto(e) => Entity::Dto(e),
            EntityMut::Rpc(e) => Entity::Rpc(e),
            EntityMut::Enum(e) => Entity::Enum(e),
            EntityMut::Field(e) => Entity::Field(e),
            EntityMut::TypeAlias(e) => Entity::TypeAlias(e),
            EntityMut::Type(e) => Entity::Type(e),
        }
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = EntityMut<'a, 'api>> {}

    pub fn ty(&self) -> EntityType {
        match self {
            EntityMut::Namespace(_) => EntityType::Namespace,
            EntityMut::Dto(_) => EntityType::Dto,
            EntityMut::Rpc(_) => EntityType::Rpc,
            EntityMut::Enum(_) => EntityType::Enum,
            EntityMut::Field(_) => EntityType::Field,
            EntityMut::Type(_) => EntityType::Type,
            EntityMut::TypeAlias(_) => EntityType::TypeAlias,
        }
    }

    /// Find an [Entity] recursively by the first instance that `predicate` returns true.
    fn find_entity<F>(self, id: EntityId, predicate: F) -> Option<(EntityId, Entity<'a, 'api>)>
    where
        F: Fn(&EntityId, &Entity<'a, 'api>) -> bool,
    {
        self.into_immutable().find_entity(id, predicate)
    }

    /// Find an [Entity] recursively by [EntityId] qualified starting with this entity.
    fn find_entity_by_id(self, id: EntityId) -> Option<Entity<'a, 'api>> {
        self.into_immutable().find_entity_by_id(id)
    }

    /// Collect all entities recursively that return true for `predicate`, not including this Entity.
    /// All collected [EntityId]s will be relative to this entity.
    fn collect_entities<F>(
        self,
        id: EntityId,
        results: &mut Vec<(EntityId, Entity<'a, 'api>)>,
        predicate: F,
    ) where
        F: Fn(&EntityId, &Entity<'a, 'api>) -> bool,
    {
        self.into_immutable()
            .collect_entities(id, results, predicate)
    }

    /// Collect all types recursively, not including this entity (if it is a type).
    /// All collected [EntityId]s will be relative to this entity.
    fn collect_types(self, id: EntityId, results: &mut Vec<(EntityId, &'a TypeRef)>) {
        self.into_immutable().collect_types(id, results)
    }

    /// Find an [Entity] recursively by the first instance that `predicate` returns true.
    fn find_entity_mut<F>(
        self,
        id: EntityId,
        predicate: F,
    ) -> Option<(EntityId, EntityMut<'a, 'api>)>
    where
        F: Fn(&EntityId, &EntityMut<'a, 'api>) -> bool,
    {
        if predicate(&id, &self) {
            return Some((id, self));
        }

        match self {
            EntityMut::Namespace(e) => e.find_entity_mut(id, predicate),
            EntityMut::Dto(e) => e.find_entity_mut(id, predicate),
            EntityMut::Rpc(e) => e.find_entity_mut(id, predicate),
            EntityMut::Enum(e) => e.find_entity_mut(id, predicate),
            EntityMut::Field(e) => e.find_entity_mut(id, predicate),
            EntityMut::TypeAlias(e) => e.find_entity_mut(id, predicate),
            EntityMut::Type(e) => e.find_entity_mut(id, predicate),
        }
    }

    /// Find an [Entity] recursively by [EntityId] qualified starting with this entity.
    fn find_entity_by_id_mut(self, id: EntityId) -> Option<EntityMut<'a, 'api>> {
        match self {
            EntityMut::Namespace(e) => e.find_entity_by_id_mut(id),
            EntityMut::Dto(e) => e.find_entity_by_id_mut(id),
            EntityMut::Rpc(e) => e.find_entity_by_id_mut(id),
            EntityMut::Enum(e) => e.find_entity_by_id_mut(id),
            EntityMut::Field(e) => e.find_entity_by_id_mut(id),
            EntityMut::TypeAlias(e) => e.find_entity_by_id_mut(id),
            EntityMut::Type(e) => e.find_entity_by_id_mut(id),
        }
    }

    /// Collect all entities recursively that return true for `predicate`, not including this Entity.
    /// All collected [EntityId]s will be relative to this entity.
    fn collect_entities_mut<F>(
        self,
        id: EntityId,
        results: &mut Vec<(EntityId, EntityMut<'a, 'api>)>,
        predicate: F,
    ) where
        F: Fn(&EntityId, &EntityMut<'a, 'api>) -> bool,
    {
        match self {
            EntityMut::Namespace(e) => e.collect_entities_mut(id, results, predicate),
            EntityMut::Dto(e) => e.collect_entities_mut(id, results, predicate),
            EntityMut::Rpc(e) => e.collect_entities_mut(id, results, predicate),
            EntityMut::Enum(e) => e.collect_entities_mut(id, results, predicate),
            EntityMut::Field(e) => e.collect_entities_mut(id, results, predicate),
            EntityMut::TypeAlias(e) => e.collect_entities_mut(id, results, predicate),
            EntityMut::Type(e) => e.collect_entities_mut(id, results, predicate),
        }
    }

    /// Collect all types recursively, not including this entity (if it is a type).
    /// All collected [EntityId]s will be relative to this entity.
    fn collect_types_mut(self, id: EntityId, results: &mut Vec<(EntityId, &'a mut TypeRef)>) {
        match self {
            EntityMut::Type(ty) => results.push((id, ty)),

            EntityMut::Namespace(e) => e.collect_types_mut(id, results),
            EntityMut::Dto(e) => e.collect_types_mut(id, results),
            EntityMut::Rpc(e) => e.collect_types_mut(id, results),
            EntityMut::Enum(e) => e.collect_types_mut(id, results),
            EntityMut::Field(e) => e.collect_types_mut(id, results),
            EntityMut::TypeAlias(e) => e.collect_types_mut(id, results),
        }
    }
}

// todo please write yourself some notes for the future...
struct ApiIterMut<'a, 'api> {
    iters: VecDeque<EntityMutIter<'a, 'api>>,
}

impl<'a, 'api> Iterator for ApiIterMut<'a, 'api> {
    type Item = EntityMut<'a, 'api>;

    fn next(&mut self) -> Option<Self::Item> {
        // Loop until we get a valid entity to return, or none.
        loop {
            match self.iters.front_mut().and_then(|iter| iter.next()) {
                Some(entity) => {
                    self.iters.push_back(entity.iter_mut()); // todo this should return an EntityMutIter
                    return Some(entity)
                },
                // Done with current entity iter, go to next iter, if available.
                None => {
                    if let None = self.iters.pop_front() {
                        return None;
                    }
                }
            }
        }
    }
}

enum EntityMutIter<'a, 'api> {
    Namespace(NamespaceEntitiesMutIter<'a, 'api>),
    Dto(DtoEntitiesMutIter<'a, 'api>),
    Rpc(RpcEntitiesMutIter<'a, 'api>),
    Enum(EnumEntitiesMutIter<'a, 'api>),
    Field(FieldEntitiesMutIter<'a, 'api>),
    TypeAlias(TypeAliasEntitiesMutIter<'a, 'api>),
    Type(TypeEntitiesMutIter<'a, 'api>),
}

impl<'a, 'api> Iterator for EntityMutIter<'a, 'api> {
    type Item = EntityMut<'a, 'api>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            EntityMutIter::Namespace(it) => it.next(),
            EntityMutIter::Dto(it) => it.next(),
            EntityMutIter::Rpc(it) => it.next(),
            EntityMutIter::Enum(it) => it.next(),
            EntityMutIter::Field(it) => it.next(),
            EntityMutIter::TypeAlias(it) => it.next(),
            EntityMutIter::Type(it) => it.next(),
        }
    }
}

fn asd() {
    let mut x = Vec::<NamespaceChild>::new();
    -- // I think I can specify the exact iterator required like this instead of resorting to generic shenanigans.
    let x: Map<IterMut<NamespaceChild>, fn(&mut NamespaceChild) -> i32> = x.iter_mut().map(|x| 0i32);
}

struct NamespaceEntitiesMutIter<'a, 'api> {
    iter: IntoIter<EntityMut<'a, 'api>>, // this might not work...
}

impl<'a, 'api> Iterator for NamespaceEntitiesMutIter<'a, 'api> {
    type Item = EntityMut<'a, 'api>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|e| e)
    }
}

struct DtoEntitiesMutIter<'a, 'api> {}

struct RpcEntitiesMutIter<'a, 'api> {}

struct EnumEntitiesMutIter<'a, 'api> {}

struct FieldEntitiesMutIter<'a, 'api> {}

struct TypeAliasEntitiesMutIter<'a, 'api> {}

struct TypeEntitiesMutIter<'a, 'api> {}

/// Supports searching and collecting entities in the entire hierarchy.
pub trait FindEntity<'api> {
    /// Find an [Entity] recursively by the first instance that `predicate` returns true.
    fn find_entity<'a, F>(
        &'a self,
        id: EntityId,
        predicate: F,
    ) -> Option<(EntityId, Entity<'a, 'api>)>
    where
        'a: 'api,
        F: Fn(&EntityId, &Entity<'a, 'api>) -> bool;

    /// Find an [Entity] recursively by [EntityId] qualified starting with this entity.
    fn find_entity_by_id<'a>(&'a self, id: EntityId) -> Option<Entity<'a, 'api>>;

    /// Collect all entities recursively that return true for `predicate`, not including this Entity.
    /// All collected [EntityId]s will be relative to this entity.
    fn collect_entities<'a, F>(
        &'a self,
        id: EntityId,
        results: &mut Vec<(EntityId, Entity<'a, 'api>)>,
        predicate: F,
    ) where
        'a: 'api,
        F: Fn(&EntityId, &Entity<'a, 'api>) -> bool;

    /// Collect all types recursively, not including this entity (if it is a type).
    /// All collected [EntityId]s will be relative to this entity.
    fn collect_types<'a>(&'a self, id: EntityId, results: &mut Vec<(EntityId, &'a TypeRef)>);
}

/// Supports searching and collecting entities in the entire hierarchy.
pub trait FindEntityMut<'api> {
    /// Find an [Entity] recursively by the first instance that `predicate` returns true.
    fn find_entity_mut<'a, F>(
        &'a mut self,
        id: EntityId,
        predicate: F,
    ) -> Option<(EntityId, EntityMut<'a, 'api>)>
    where
        'a: 'api,
        F: Fn(&EntityId, &EntityMut<'a, 'api>) -> bool;

    /// Find an [Entity] recursively by [EntityId] qualified starting with this entity.
    fn find_entity_by_id_mut<'a>(&'a mut self, id: EntityId) -> Option<EntityMut<'a, 'api>>;

    /// Collect all entities recursively that return true for `predicate`, not including this Entity.
    /// All collected [EntityId]s will be relative to this entity.
    fn collect_entities_mut<'a, F>(
        &'a mut self,
        id: EntityId,
        results: &mut Vec<(EntityId, EntityMut<'a, 'api>)>,
        predicate: F,
    ) where
        'a: 'api,
        F: Fn(&EntityId, &EntityMut<'a, 'api>) -> bool;

    /// Collect all types recursively, not including this entity (if it is a type).
    /// All collected [EntityId]s will be relative to this entity.
    fn collect_types_mut<'a>(
        &'a mut self,
        id: EntityId,
        results: &mut Vec<(EntityId, &'a mut TypeRef)>,
    );
}

pub trait AsEntity<'api> {
    /// Create an [Entity] reference to this entity.
    fn as_entity<'a>(&'a self) -> Entity<'a, 'api>;

    /// Create an [EntityMut] reference to this entity.
    fn as_entity_mut<'a>(&'a mut self) -> EntityMut<'a, 'api>;

    fn entity_type(&self) -> EntityType {
        self.as_entity().ty()
    }
}

// Keep these in line with [EntityId] documentation.
#[rustfmt::skip]
pub mod subtype {
    pub const NAMESPACE: &str =             "namespace";
    pub const NAMESPACE_SHORTISH: &str =    "ns";
    pub const NAMESPACE_SHORT: &str =       "n";
    pub const DTO: &str =                   "dto";
    pub const DTO_SHORT: &str =             "d";
    pub const RPC: &str =                   "rpc";
    pub const RPC_SHORT: &str =             "r";
    pub const ENUM: &str =                  "enum";
    pub const ENUM_MED: &str =              "en";
    pub const ENUM_SHORT: &str =            "e";
    pub const FIELD: &str =                 "field";
    pub const FIELD_SHORT: &str =           "f";
    pub const PARAM: &str =                 "param";
    pub const PARAM_SHORT: &str =           "p";
    pub const TY: &str =                    "ty";
    pub const RETURN_TY: &str =             "return_ty";
    pub const TY_ALIAS: &str =              "alias";
    pub const TY_ALIAS_SHORT: &str =        "a";
    pub const TY_ALIAS_TARGET: &str =       "target_ty";

    pub const NAMESPACE_ALL: &[&str] = &[NAMESPACE, NAMESPACE_SHORTISH, NAMESPACE_SHORT];
    pub const DTO_ALL: &[&str] = &[DTO, DTO_SHORT];
    pub const RPC_ALL: &[&str] = &[RPC, RPC_SHORT];
    pub const ENUM_ALL: &[&str] = &[ENUM, ENUM_MED, ENUM_SHORT];
    pub const FIELD_ALL: &[&str] = &[FIELD, FIELD_SHORT];
    pub const PARAM_ALL: &[&str] = &[PARAM, PARAM_SHORT];
    pub const TY_ALL: &[&str] = &[TY];
    pub const RETURN_TY_ALL: &[&str] = &[RETURN_TY];
    pub const TY_ALIAS_ALL: &[&str] = &[TY_ALIAS, TY_ALIAS_SHORT];
    pub const TY_ALIAS_TARGET_ALL: &[&str] = &[TY_ALIAS_TARGET];
}

impl EntityType {
    pub fn is_valid_subtype(&self, ty: &EntityType) -> bool {
        // Keep these in line with [EntityId] documentation.
        // All variants are specified so additions to enum will force consideration here.
        match self {
            EntityType::None => *ty == EntityType::None,

            EntityType::Namespace => match ty {
                EntityType::Namespace
                | EntityType::Dto
                | EntityType::Rpc
                | EntityType::Enum
                | EntityType::TypeAlias
                | EntityType::Field => true,
                EntityType::Type | EntityType::None => false,
            },

            EntityType::Dto => match ty {
                EntityType::Field
                | EntityType::Dto
                | EntityType::Rpc
                | EntityType::TypeAlias
                | EntityType::Enum => true,

                EntityType::Namespace | EntityType::Type | EntityType::None => false,
            },

            EntityType::Rpc => match ty {
                EntityType::Field | EntityType::Type => true,
                EntityType::Namespace
                | EntityType::Dto
                | EntityType::Rpc
                | EntityType::Enum
                | EntityType::TypeAlias
                | EntityType::None => false,
            },

            EntityType::Enum => match ty {
                EntityType::Namespace
                | EntityType::Dto
                | EntityType::Rpc
                | EntityType::Enum
                | EntityType::Type
                | EntityType::TypeAlias
                | EntityType::Field
                | EntityType::None => false,
            },

            EntityType::Field => match ty {
                EntityType::Type => true,
                EntityType::Namespace
                | EntityType::Dto
                | EntityType::Rpc
                | EntityType::Enum
                | EntityType::TypeAlias
                | EntityType::Field
                | EntityType::None => false,
            },

            EntityType::Type => match ty {
                EntityType::Namespace
                | EntityType::Dto
                | EntityType::Rpc
                | EntityType::Enum
                | EntityType::Type
                | EntityType::TypeAlias
                | EntityType::Field
                | EntityType::None => false,
            },

            EntityType::TypeAlias => match ty {
                EntityType::Type => true,
                EntityType::Namespace
                | EntityType::Dto
                | EntityType::Rpc
                | EntityType::Enum
                | EntityType::TypeAlias
                | EntityType::Field
                | EntityType::None => false,
            },
        }
    }
}

impl TryFrom<&str> for EntityType {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            _ if subtype::NAMESPACE_ALL.contains(&value) => Ok(EntityType::Namespace),
            _ if subtype::DTO_ALL.contains(&value) => Ok(EntityType::Dto),
            _ if subtype::RPC_ALL.contains(&value) => Ok(EntityType::Rpc),
            _ if subtype::ENUM_ALL.contains(&value) => Ok(EntityType::Enum),
            _ if subtype::FIELD_ALL.contains(&value) => Ok(EntityType::Field),
            _ if subtype::PARAM_ALL.contains(&value) => Ok(EntityType::Field),
            _ if subtype::TY_ALL.contains(&value) => Ok(EntityType::Type),
            _ if subtype::RETURN_TY_ALL.contains(&value) => Ok(EntityType::Type),
            _ if subtype::TY_ALIAS_TARGET_ALL.contains(&value) => Ok(EntityType::Type),
            _ if subtype::TY_ALIAS_ALL.contains(&value) => Ok(EntityType::TypeAlias),
            _ => Err(anyhow!(
                "subtype '{}' does not map to a valid EntityType",
                value
            )),
        }
    }
}

/// A set of macros to simplify adding tests for common FindEntity cases.
#[cfg(test)]
pub mod test_macros {
    macro_rules! find_entity_match_predicate {
        ($var_name:ident, $entity_type:ident) => {
            let expected_id = EntityId::new_unqualified("something");
            let (id, entity) = $var_name
                .find_entity(expected_id.clone(), |_, x| {
                    x.ty() == EntityType::$entity_type
                })
                .unwrap();
            assert_eq!(id, expected_id);
            assert_eq!(entity.ty(), EntityType::$entity_type);
            if let Entity::$entity_type(actual) = entity {
                assert_eq!(&$var_name, actual);
            }
        };
    }
    pub(crate) use find_entity_match_predicate;

    macro_rules! find_entity_no_match_predicate {
        ($var_name:ident, $wrong_entity_type:ident) => {
            let result = $var_name.find_entity(EntityId::default(), |_, x| {
                x.ty() == EntityType::$wrong_entity_type
            });
            assert!(result.is_none());
        };
    }
    pub(crate) use find_entity_no_match_predicate;

    macro_rules! find_entity_by_id_found {
        ($var_name:ident, $entity_type:ident) => {
            // Empty id indicates this is the find target.
            let id = EntityId::default();
            let entity = $var_name.find_entity_by_id(id).unwrap();
            assert_eq!(entity.ty(), EntityType::$entity_type);
            if let Entity::$entity_type(actual) = entity {
                assert_eq!(&$var_name, actual);
            };
        };
    }
    pub(crate) use find_entity_by_id_found;

    macro_rules! find_entity_by_id_not_found {
        ($var_name:ident) => {
            // Any components left in entity id == still more to search
            let id = EntityId::new_unqualified("anything");
            let result = $var_name.find_entity_by_id(id);
            assert!(result.is_none());
        };
    }
    pub(crate) use find_entity_by_id_not_found;

    macro_rules! collect_entities_match_predicate {
        ($var_name:ident, $entity_type:ident) => {
            let id = EntityId::default();
            let mut collected = Vec::new();

            $var_name.collect_entities(id, &mut collected, |_, x| {
                x.ty() == EntityType::$entity_type
            });
            assert!(!collected.is_empty());

            let (_, entity) = collected.first().unwrap();
            assert_eq!(entity.ty(), EntityType::$entity_type);
            if let Entity::$entity_type(actual) = entity {
                assert_eq!(&$var_name, *actual);
            };
        };
    }
    pub(crate) use collect_entities_match_predicate;

    macro_rules! collect_entities_no_match_predicate {
        ($var_name:ident, $wrong_entity_type:ident) => {
            let id = EntityId::default();
            let mut collected = Vec::new();
            $var_name.collect_entities(id, &mut collected, |_, x| {
                x.ty() == EntityType::$wrong_entity_type
            });
            assert!(collected.is_empty());
        };
    }
    pub(crate) use collect_entities_no_match_predicate;
}
