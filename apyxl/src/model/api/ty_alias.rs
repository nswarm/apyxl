use crate::model::attributes::AttributesHolder;
use crate::model::entity::{EntityMut, FindEntity, ToEntity};
use crate::model::{entity, Attributes, Entity, EntityId, EntityType, TypeRef};
use anyhow::anyhow;

/// A single enum type in the within an [Api].
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TypeAlias<'a> {
    pub name: &'a str,
    pub target_ty: TypeRef,
    pub attributes: Attributes<'a>,
}

impl ToEntity for TypeAlias<'_> {
    fn to_entity(&self) -> Entity {
        Entity::TypeAlias(self)
    }
}

impl AttributesHolder for TypeAlias<'_> {
    fn attributes(&self) -> &Attributes {
        &self.attributes
    }
}

impl<'api> FindEntity<'api> for TypeAlias<'api> {
    fn qualify_id(&self, mut id: EntityId, referenceable: bool) -> anyhow::Result<EntityId> {
        match id.pop_front() {
            None => Ok(EntityId::default()),
            Some((_, name)) => {
                if referenceable {
                    return Err(anyhow!(
                        "failed to qualify_id {}, type alias has no referenceable types",
                        name
                    ));
                }
                if name == entity::subtype::TY_ALIAS_TARGET {
                    Ok(EntityId::new(
                        EntityType::Type,
                        entity::subtype::TY_ALIAS_TARGET,
                    ))
                } else {
                    Err(anyhow!(
                        "failed to qualify_id: {} is an invalid field child",
                        name
                    ))
                }
            }
        }
    }

    fn find_entity<'a>(&'a self, mut id: EntityId) -> Option<Entity<'a, 'api>> {
        if let Some((ty, name)) = id.pop_front() {
            match ty {
                EntityType::Type => {
                    if entity::subtype::TY_ALIAS_TARGET_ALL.contains(&name.as_str()) {
                        Some(Entity::Type(&self.target_ty))
                    } else {
                        None
                    }
                }

                EntityType::None
                | EntityType::Namespace
                | EntityType::Dto
                | EntityType::Rpc
                | EntityType::Enum
                | EntityType::TypeAlias
                | EntityType::Field => None,
            }
        } else {
            Some(Entity::TypeAlias(self))
        }
    }

    fn find_entity_mut<'a>(&'a mut self, mut id: EntityId) -> Option<EntityMut<'a, 'api>> {
        if let Some((ty, name)) = id.pop_front() {
            match ty {
                EntityType::Type => {
                    if entity::subtype::TY_ALIAS_TARGET_ALL.contains(&name.as_str()) {
                        Some(EntityMut::Type(&mut self.target_ty))
                    } else {
                        None
                    }
                }

                EntityType::None
                | EntityType::Namespace
                | EntityType::Dto
                | EntityType::Rpc
                | EntityType::Enum
                | EntityType::TypeAlias
                | EntityType::Field => None,
            }
        } else {
            Some(EntityMut::TypeAlias(self))
        }
    }
}
