use anyhow::anyhow;

use crate::model::{Dto, EntityId, Enum, Field, Namespace, Rpc, TypeAlias, TypeRef};

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
pub enum EntityMut<'a, 'api> {
    Namespace(&'a mut Namespace<'api>),
    Dto(&'a mut Dto<'api>),
    Rpc(&'a mut Rpc<'api>),
    Enum(&'a mut Enum<'api>),
    Field(&'a mut Field<'api>),
    TypeAlias(&'a mut TypeAlias<'api>),
    Type(&'a mut TypeRef),
}

pub trait FindEntity<'api> {
    /// Find an [Entity] by qualified [EntityId], if it exists.
    fn find_entity<'a>(&'a self, id: EntityId) -> Option<Entity<'a, 'api>>;

    /// Find an [Entity] by qualified [EntityId], if it exists.
    fn find_entity_mut<'a>(&'a mut self, id: EntityId) -> Option<EntityMut<'a, 'api>>;
}

impl<'api> FindEntity<'api> for Entity<'_, 'api> {
    fn find_entity<'a>(&'a self, id: EntityId) -> Option<Entity<'a, 'api>> {
        match self {
            Entity::Namespace(ns) => ns.find_entity(id),
            Entity::Dto(dto) => dto.find_entity(id),
            Entity::Rpc(rpc) => rpc.find_entity(id),
            Entity::Enum(en) => en.find_entity(id),
            Entity::Field(field) => field.find_entity(id),
            Entity::TypeAlias(alias) => alias.find_entity(id),
            Entity::Type(ty) => ty.find_entity(id),
        }
    }

    fn find_entity_mut<'a>(&'a mut self, _: EntityId) -> Option<EntityMut<'a, 'api>> {
        panic!("cannot find mut through immutable Entity reference")
    }
}

impl<'api> FindEntity<'api> for EntityMut<'_, 'api> {
    fn find_entity<'a>(&'a self, id: EntityId) -> Option<Entity<'a, 'api>> {
        match self {
            EntityMut::Namespace(ns) => ns.find_entity(id),
            EntityMut::Dto(dto) => dto.find_entity(id),
            EntityMut::Rpc(rpc) => rpc.find_entity(id),
            EntityMut::Enum(en) => en.find_entity(id),
            EntityMut::Field(field) => field.find_entity(id),
            EntityMut::TypeAlias(alias) => alias.find_entity(id),
            EntityMut::Type(ty) => ty.find_entity(id),
        }
    }

    fn find_entity_mut<'a>(&'a mut self, id: EntityId) -> Option<EntityMut<'a, 'api>> {
        match self {
            EntityMut::Namespace(ns) => ns.find_entity_mut(id),
            EntityMut::Dto(dto) => dto.find_entity_mut(id),
            EntityMut::Rpc(rpc) => rpc.find_entity_mut(id),
            EntityMut::Enum(en) => en.find_entity_mut(id),
            EntityMut::Field(field) => field.find_entity_mut(id),
            EntityMut::TypeAlias(alias) => alias.find_entity_mut(id),
            EntityMut::Type(ty) => ty.find_entity_mut(id),
        }
    }
}

pub trait ToEntity {
    /// Create an [Entity] reference to this entity.
    fn to_entity(&self) -> Entity;

    fn entity_type(&self) -> EntityType {
        self.to_entity().ty()
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
                EntityType::Field | EntityType::Dto | EntityType::Rpc | EntityType::TypeAlias => {
                    true
                }

                EntityType::Namespace | EntityType::Enum | EntityType::Type | EntityType::None => {
                    false
                }
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

impl Entity<'_, '_> {
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
