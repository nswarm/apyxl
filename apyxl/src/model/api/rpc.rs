use crate::model::api::entity::ToEntity;
use crate::model::entity::{EntityMut, FindEntity};
use crate::model::{entity, Attributes, Entity, EntityId, EntityType, Field, Type};

/// A single Remote Procedure Call (RPC) within an [Api].
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct Rpc<'a> {
    pub name: &'a str,
    pub params: Vec<Field<'a>>,
    pub return_type: Option<Type>,
    pub attributes: Attributes,
}

impl<'a> Rpc<'a> {
    pub fn param(&self, name: &str) -> Option<&Field<'a>> {
        self.params.iter().find(|param| param.name == name)
    }

    pub fn param_mut(&mut self, name: &str) -> Option<&mut Field<'a>> {
        self.params.iter_mut().find(|param| param.name == name)
    }
}

impl ToEntity for Rpc<'_> {
    fn to_entity(&self) -> Entity {
        Entity::Rpc(self)
    }
}

impl<'api> FindEntity<'api> for Rpc<'api> {
    fn find_entity<'a>(&'a self, mut id: EntityId) -> Option<Entity<'a, 'api>> {
        if let Some((ty, name)) = id.pop_front() {
            match ty {
                EntityType::Field => self.param(&name).map_or(None, |x| x.find_entity(id)),

                EntityType::Type => {
                    if entity::subtype::RETURN_TY_ALL.contains(&name.as_str()) {
                        self.return_type.as_ref().map(Entity::Type)
                    } else {
                        None
                    }
                }

                EntityType::None
                | EntityType::Namespace
                | EntityType::Dto
                | EntityType::Rpc
                | EntityType::Enum => None,
            }
        } else {
            Some(Entity::Rpc(self))
        }
    }

    fn find_entity_mut<'a>(&'a mut self, mut id: EntityId) -> Option<EntityMut<'a, 'api>> {
        if let Some((ty, name)) = id.pop_front() {
            match ty {
                EntityType::Field => self
                    .param_mut(&name)
                    .map_or(None, |x| x.find_entity_mut(id)),

                EntityType::Type => {
                    if entity::subtype::RETURN_TY_ALL.contains(&name.as_str()) {
                        self.return_type.as_mut().map(EntityMut::Type)
                    } else {
                        None
                    }
                }

                EntityType::None
                | EntityType::Namespace
                | EntityType::Dto
                | EntityType::Rpc
                | EntityType::Enum => None,
            }
        } else {
            Some(EntityMut::Rpc(self))
        }
    }
}
