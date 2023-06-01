use crate::model::entity::{EntityMut, FindEntity};
use crate::model::{entity, Attributes, Entity, EntityId, EntityType, Type};

/// A pair of name and type that describe a named instance of a type e.g. within a [Dto] or [Rpc].
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Field<'a> {
    pub name: &'a str,
    pub ty: Type,
    pub attributes: Attributes<'a>,
}

impl<'api> FindEntity<'api> for Field<'api> {
    fn find_entity<'a>(&'a self, mut id: EntityId) -> Option<Entity<'a, 'api>> {
        if let Some((ty, name)) = id.pop_front() {
            match ty {
                EntityType::Type => {
                    if entity::subtype::TY_ALL.contains(&name.as_str()) {
                        Some(Entity::Type(&self.ty))
                    } else {
                        None
                    }
                }

                EntityType::None
                | EntityType::Namespace
                | EntityType::Dto
                | EntityType::Rpc
                | EntityType::Enum
                | EntityType::Field => None,
            }
        } else {
            Some(Entity::Field(self))
        }
    }

    fn find_entity_mut<'a>(&'a mut self, mut id: EntityId) -> Option<EntityMut<'a, 'api>> {
        if let Some((ty, name)) = id.pop_front() {
            match ty {
                EntityType::Type => {
                    if entity::subtype::TY_ALL.contains(&name.as_str()) {
                        Some(EntityMut::Type(&mut self.ty))
                    } else {
                        None
                    }
                }

                EntityType::None
                | EntityType::Namespace
                | EntityType::Dto
                | EntityType::Rpc
                | EntityType::Enum
                | EntityType::Field => None,
            }
        } else {
            Some(EntityMut::Field(self))
        }
    }
}
