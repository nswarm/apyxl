use crate::model::api::entity::ToEntity;
use crate::model::attributes::AttributesHolder;
use crate::model::entity::{EntityMut, FindEntity};
use crate::model::{Attributes, Entity, EntityId, EntityType, Field, Namespace, Rpc};

/// A single Data Transfer Object (DTO) used in an [Rpc], either directly or nested in another [Dto].
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct Dto<'a> {
    pub name: &'a str,
    pub fields: Vec<Field<'a>>,
    pub rpcs: Vec<Rpc<'a>>,
    pub attributes: Attributes<'a>,

    /// Namespace that holds e.g. nested [Dtos], [Rpcs], and [TypeAliases].
    pub namespace: Option<Namespace<'a>>,
}

impl<'a> Dto<'a> {
    pub fn field(&self, name: &str) -> Option<&Field<'a>> {
        self.fields.iter().find(|field| field.name == name)
    }

    pub fn field_mut(&mut self, name: &str) -> Option<&mut Field<'a>> {
        self.fields.iter_mut().find(|field| field.name == name)
    }

    pub fn rpc(&self, name: &str) -> Option<&Rpc<'a>> {
        self.rpcs.iter().find(|rpc| rpc.name == name)
    }

    pub fn rpc_mut(&mut self, name: &str) -> Option<&mut Rpc<'a>> {
        self.rpcs.iter_mut().find(|rpc| rpc.name == name)
    }
}

impl ToEntity for Dto<'_> {
    fn to_entity(&self) -> Entity {
        Entity::Dto(self)
    }
}

impl AttributesHolder for Dto<'_> {
    fn attributes(&self) -> &Attributes {
        &self.attributes
    }
}

impl<'api> FindEntity<'api> for Dto<'api> {
    fn find_entity<'a>(&'a self, mut id: EntityId) -> Option<Entity<'a, 'api>> {
        if let Some((ty, name)) = id.pop_front() {
            // Need to put the id for this level back together and evaluate it on the namespace
            // if necessary.
            let static_id = EntityId::default()
                .child(ty, &name)
                .unwrap()
                .concat(&id)
                .unwrap();
            match ty {
                EntityType::Field => self
                    .field(&name)
                    .and_then(|x| x.find_entity(id.clone()))
                    .or_else(|| self.namespace.as_ref()?.find_entity(static_id)),

                EntityType::Rpc => self
                    .rpc(&name)
                    .and_then(|x| x.find_entity(id.clone()))
                    .or_else(|| self.namespace.as_ref()?.find_entity(static_id)),

                EntityType::Dto | EntityType::TypeAlias => {
                    self.namespace.as_ref()?.find_entity(static_id)
                }

                EntityType::None | EntityType::Namespace | EntityType::Enum | EntityType::Type => {
                    None
                }
            }
        } else {
            Some(Entity::Dto(self))
        }
    }

    fn find_entity_mut<'a>(&'a mut self, mut id: EntityId) -> Option<EntityMut<'a, 'api>> {
        if let Some((ty, name)) = id.pop_front() {
            // Need to put the id for this level back together and evaluate it on the namespace
            // if necessary.
            let static_id = EntityId::default()
                .child(ty, &name)
                .unwrap()
                .concat(&id)
                .unwrap();
            match ty {
                EntityType::Field => {
                    let is_member = self.field(&name).is_some();
                    if is_member {
                        self.field_mut(&name)
                            .and_then(|x| x.find_entity_mut(id.clone()))
                    } else {
                        self.namespace.as_mut()?.find_entity_mut(static_id)
                    }
                }

                EntityType::Rpc => {
                    let is_member = self.rpc(&name).is_some();
                    if is_member {
                        self.rpc_mut(&name)
                            .and_then(|x| x.find_entity_mut(id.clone()))
                    } else {
                        self.namespace.as_mut()?.find_entity_mut(static_id)
                    }
                }

                EntityType::Dto | EntityType::TypeAlias => {
                    self.namespace.as_mut()?.find_entity_mut(static_id)
                }

                EntityType::None | EntityType::Namespace | EntityType::Enum | EntityType::Type => {
                    None
                }
            }
        } else {
            Some(EntityMut::Dto(self))
        }
    }
}
