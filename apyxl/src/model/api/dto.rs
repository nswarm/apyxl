use crate::model::api::entity::ToEntity;
use crate::model::entity::{EntityMut, FindEntity};
use crate::model::{Attributes, Entity, EntityId, EntityType, Field, Namespace};
use crate::model::attributes::AttributesHolder;

/// A single Data Transfer Object (DTO) used in an [Rpc], either directly or nested in another [Dto].
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct Dto<'a> {
    pub name: &'a str,
    pub fields: Vec<Field<'a>>,
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
            match ty {
                EntityType::Field => self.field(&name).and_then(|x| x.find_entity(id)),

                EntityType::Dto | EntityType::Rpc | EntityType::TypeAlias => {
                    // Need to put the id for this level back together and evaluate it on the namespace.
                    let id = EntityId::default()
                        .child(ty, name)
                        .unwrap()
                        .concat(&id)
                        .unwrap();
                    self.namespace.as_ref()?.find_entity(id)
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
            match ty {
                EntityType::Field => self.field_mut(&name).and_then(|x| x.find_entity_mut(id)),

                EntityType::Dto | EntityType::Rpc | EntityType::TypeAlias => {
                    // Need to put the id for this level back together and evaluate it on the namespace.
                    let id = EntityId::default()
                        .child(ty, name)
                        .unwrap()
                        .concat(&id)
                        .unwrap();
                    self.namespace.as_mut()?.find_entity_mut(id)
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
