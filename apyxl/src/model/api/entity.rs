use crate::model::{Dto, EntityId, Enum, Field, Namespace, Rpc, Type};
use anyhow::anyhow;

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Copy, Clone)]
pub enum EntityType {
    None, // Unqualified EntityIds.
    Namespace,
    Dto,
    Rpc,
    Enum,
    Field,
    Type,
}

/// Reference to a specific entity within an API.
pub enum Entity<'a> {
    Namespace(&'a Namespace<'a>),
    Dto(&'a Dto<'a>),
    Rpc(&'a Rpc<'a>),
    Enum(&'a Enum<'a>),
    Field(&'a Field<'a>),
    Type(&'a Type),
}

pub trait ToEntity {
    /// Create an [Entity] reference to this entity.
    fn to_entity(&self) -> Entity;

    fn entity_type(&self) -> EntityType {
        self.to_entity().ty()
    }
}

pub trait FindEntity {
    /// Find an [Entity] by [EntityId], if it exists.
    fn find_entity(&self, id: &EntityId) -> Option<Entity>;
}

// Keep these in line with [EntityId] documentation.
#[rustfmt::skip]
pub mod subtype {
    pub const NAMESPACE: &str =       "namespace";
    pub const NAMESPACE_SHORT: &str = "n";
    pub const DTO: &str =             "dto";
    pub const DTO_SHORT: &str =       "d";
    pub const RPC: &str =             "rpc";
    pub const RPC_SHORT: &str =       "r";
    pub const ENUM: &str =            "enum";
    pub const ENUM_MED: &str =        "en";
    pub const ENUM_SHORT: &str =      "e";
    pub const FIELD: &str =           "field";
    pub const FIELD_SHORT: &str =     "f";
    pub const PARAM: &str =           "param";
    pub const PARAM_SHORT: &str =     "p";
    pub const TY: &str =              "ty";
    pub const RETURN_TY: &str =       "return_ty";

    pub const NAMESPACE_ALL: &[&str] = &[NAMESPACE, NAMESPACE_SHORT];
    pub const DTO_ALL: &[&str] = &[DTO, DTO_SHORT];
    pub const RPC_ALL: &[&str] = &[RPC, RPC_SHORT];
    pub const ENUM_ALL: &[&str] = &[ENUM, ENUM_MED, ENUM_SHORT];
    pub const FIELD_ALL: &[&str] = &[FIELD, FIELD_SHORT];
    pub const PARAM_ALL: &[&str] = &[PARAM, PARAM_SHORT];
    pub const TY_ALL: &[&str] = &[TY];
    pub const RETURN_TY_ALL: &[&str] = &[RETURN_TY];
}

impl EntityType {
    pub fn is_valid_subtype(&self, ty: &EntityType) -> bool {
        // Keep these in line with [EntityId] documentation.
        // All variants are specified so additions to enum will force consideration here.
        match self {
            EntityType::None => *ty == EntityType::None,

            EntityType::Namespace => match ty {
                EntityType::Namespace | EntityType::Dto | EntityType::Rpc | EntityType::Enum => {
                    true
                }
                EntityType::Field | EntityType::Type | EntityType::None => false,
            },

            EntityType::Dto => match ty {
                EntityType::Field => true,
                EntityType::Namespace
                | EntityType::Dto
                | EntityType::Rpc
                | EntityType::Enum
                | EntityType::Type
                | EntityType::None => false,
            },

            EntityType::Rpc => match ty {
                EntityType::Field | EntityType::Type => true,
                EntityType::Namespace
                | EntityType::Dto
                | EntityType::Rpc
                | EntityType::Enum
                | EntityType::None => false,
            },

            EntityType::Enum => match ty {
                EntityType::Namespace
                | EntityType::Dto
                | EntityType::Rpc
                | EntityType::Enum
                | EntityType::Type
                | EntityType::Field
                | EntityType::None => false,
            },

            EntityType::Field => match ty {
                EntityType::Type => true,
                EntityType::Namespace
                | EntityType::Dto
                | EntityType::Rpc
                | EntityType::Enum
                | EntityType::Field
                | EntityType::None => false,
            },

            EntityType::Type => match ty {
                EntityType::Namespace
                | EntityType::Dto
                | EntityType::Rpc
                | EntityType::Enum
                | EntityType::Type
                | EntityType::Field
                | EntityType::None => false,
            },
        }
    }
}

impl Entity<'_> {
    pub fn ty(&self) -> EntityType {
        match self {
            Entity::Namespace(_) => EntityType::Namespace,
            Entity::Dto(_) => EntityType::Dto,
            Entity::Rpc(_) => EntityType::Rpc,
            Entity::Enum(_) => EntityType::Enum,
            Entity::Field(_) => EntityType::Field,
            Entity::Type(_) => EntityType::Type,
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
            _ => Err(anyhow!(
                "subtype '{}' does not map to a valid EntityType",
                value
            )),
        }
    }
}
