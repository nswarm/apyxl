use crate::model::api::entity::ToEntity;
use crate::model::attributes::AttributesHolder;
use crate::model::entity::{EntityMut, FindEntity};
use crate::model::{Attributes, Entity, EntityId};

/// A single enum type in the within an [Api].
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct Enum<'a> {
    pub name: &'a str,
    pub values: Vec<EnumValue<'a>>,
    pub attributes: Attributes<'a>,
}

pub type EnumValueNumber = i64;

/// A single value within an [Enum].
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct EnumValue<'a> {
    pub name: &'a str,
    pub number: EnumValueNumber,
    pub attributes: Attributes<'a>,
}

impl<'a> Enum<'a> {
    pub fn value(&self, name: &str) -> Option<&EnumValue<'a>> {
        self.values.iter().find(|value| value.name == name)
    }

    pub fn value_mut(&mut self, name: &str) -> Option<&mut EnumValue<'a>> {
        self.values.iter_mut().find(|value| value.name == name)
    }
}

impl ToEntity for Enum<'_> {
    fn to_entity(&self) -> Entity {
        Entity::Enum(self)
    }
}

impl AttributesHolder for Enum<'_> {
    fn attributes(&self) -> &Attributes {
        &self.attributes
    }
}

impl<'api> FindEntity<'api> for Enum<'api> {
    fn find_entity<'a>(&'a self, id: EntityId) -> Option<Entity<'a, 'api>> {
        if id.is_empty() {
            Some(Entity::Enum(self))
        } else {
            None
        }
    }

    fn find_entity_mut<'a>(&'a mut self, id: EntityId) -> Option<EntityMut<'a, 'api>> {
        if id.is_empty() {
            Some(EntityMut::Enum(self))
        } else {
            None
        }
    }
}
