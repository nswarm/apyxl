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
pub enum Entity<'a, 'api> {
    Namespace(&'a Namespace<'api>),
    Dto(&'a Dto<'api>),
    Rpc(&'a Rpc<'api>),
    Enum(&'a Enum<'api>),
    Field(&'a Field<'api>),
    Type(&'a Type),
}

/// Mutable reference to a specific entity within an API.
pub enum EntityMut<'a, 'api> {
    Namespace(&'a mut Namespace<'api>),
    Dto(&'a mut Dto<'api>),
    Rpc(&'a mut Rpc<'api>),
    Enum(&'a mut Enum<'api>),
    Field(&'a mut Field<'api>),
    Type(&'a mut Type),
}

/// Find an entity mutably by qualified [EntityId].
// pub fn find_entity_mut<'a, 'b>(
//     ns: &'b mut Namespace<'a>,
//     mut id: EntityId,
// ) -> Option<EntityMut<'a, 'b>> {
//     // This is a free fn to avoid recursive mutable self references.
//     let mut entity = EntityMut::Namespace(ns);
//     while let Some((ty, name)) = id.pop_front() {
//         match (entity, ty) {
//             (EntityMut::Namespace(ns), EntityType::Namespace) => match ns.namespace_mut(&name) {
//                 None => return None,
//                 Some(value) => entity = EntityMut::Namespace(value),
//             },
//             (EntityMut::Namespace(ns), EntityType::Dto) => match ns.dto_mut(&name) {
//                 None => return None,
//                 Some(value) => entity = EntityMut::Dto(value),
//             },
//             (EntityMut::Namespace(ns), EntityType::Rpc) => match ns.rpc_mut(&name) {
//                 None => return None,
//                 Some(value) => entity = EntityMut::Rpc(value),
//             },
//             _ => return None,
//         }
//     }
//     Some(entity)
// }

// pub fn mutate_entity<'a>(ns: &'a mut Namespace<'a>, mut id: EntityId, ) {
//     // This is a free fn to avoid recursive mutable self references.
//     let mut entity = EntityMut::Namespace(ns);
//     while let Some((ty, name)) = id.pop_front() {
//         match (entity, ty) {
//             (EntityMut::Namespace(ns), EntityType::Namespace) => match ns.namespace_mut(&name) {
//                 None => return None,
//                 Some(value) => entity = EntityMut::Namespace(value),
//             },
//             (EntityMut::Namespace(ns), EntityType::Dto) => match ns.dto_mut(&name) {
//                 None => return None,
//                 Some(value) => entity = EntityMut::Dto(value),
//             },
//             (EntityMut::Namespace(ns), EntityType::Rpc) => match ns.rpc_mut(&name) {
//                 None => return None,
//                 Some(value) => entity = EntityMut::Rpc(value),
//             },
//             _ => return None,
//         }
//     }
//     Some(entity)
// }

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
            Entity::Enum(_) => None,
            Entity::Field(field) => field.find_entity(id),
            Entity::Type(_) => None,
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
            EntityMut::Type(_) => None,
        }
    }

    fn find_entity_mut<'a>(&'a mut self, id: EntityId) -> Option<EntityMut<'a, 'api>> {
        match self {
            EntityMut::Namespace(ns) => ns.find_entity_mut(id),
            EntityMut::Dto(dto) => dto.find_entity_mut(id),
            EntityMut::Rpc(rpc) => rpc.find_entity_mut(id),
            EntityMut::Enum(en) => en.find_entity_mut(id),
            EntityMut::Field(field) => field.find_entity_mut(id),
            EntityMut::Type(_) => None,
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

impl Entity<'_, '_> {
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
