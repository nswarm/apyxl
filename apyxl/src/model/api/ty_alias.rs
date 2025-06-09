use crate::model::attributes::AttributesHolder;
use crate::model::entity::{AsEntity, EntityMut, FindEntity, FindEntityMut};
use crate::model::{entity, Attributes, Entity, EntityId, EntityType, TypeRef};

/// A single enum type in the within an [Api].
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TypeAlias<'a> {
    pub name: &'a str,
    pub target_ty: TypeRef,
    pub attributes: Attributes<'a>,
}

impl<'api> AsEntity<'api> for TypeAlias<'api> {
    fn as_entity<'a>(&'a self) -> Entity<'a, 'api> {
        Entity::TypeAlias(self)
    }

    fn as_entity_mut<'a>(&'a mut self) -> EntityMut<'a, 'api> {
        EntityMut::TypeAlias(self)
    }
}

impl AttributesHolder for TypeAlias<'_> {
    fn attributes(&self) -> &Attributes {
        &self.attributes
    }
}

impl<'api> FindEntity<'api> for TypeAlias<'api> {
    fn find_entity<'a, F>(
        &'a self,
        id: EntityId,
        predicate: F,
    ) -> Option<(EntityId, Entity<'a, 'api>)>
    where
        'a: 'api,
        F: Fn(&EntityId, &Entity<'a, 'api>) -> bool,
    {
        if predicate(&id, &self.as_entity()) {
            Some((id, self.as_entity()))
        } else {
            let child_id = id
                .child(EntityType::Type, entity::subtype::TY_ALIAS_TARGET)
                .unwrap();
            self.target_ty.find_entity(child_id, predicate)
        }
    }

    fn find_entity_by_id<'a>(&'a self, mut id: EntityId) -> Option<Entity<'a, 'api>> {
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

    fn collect_entities<'a, F>(
        &'a self,
        id: EntityId,
        results: &mut Vec<(EntityId, Entity<'a, 'api>)>,
        predicate: F,
    ) where
        'a: 'api,
        F: Fn(&EntityId, &Entity<'a, 'api>) -> bool,
    {
        todo!()
    }

    fn collect_types(&self, id: EntityId, results: &mut Vec<(EntityId, &TypeRef)>) {
        todo!()
    }
}

impl<'api> FindEntityMut<'api> for TypeAlias<'api> {
    fn find_entity_mut<'a, F>(
        &'a mut self,
        id: EntityId,
        predicate: F,
    ) -> Option<(EntityId, EntityMut<'a, 'api>)>
    where
        'a: 'api,
        F: for<'p> Fn(&'p EntityId, &'p EntityMut<'a, 'api>) -> bool,
    {
        todo!()
        // let entity = self.as_entity_mut();
        // if predicate(&id, &entity) {
        //     return Some((id, entity));
        // }
        //
        // let child_id = id
        //     .child(EntityType::Type, entity::subtype::TY_ALIAS_TARGET)
        //     .unwrap();
        // self.target_ty.find_entity_mut(child_id, predicate)
    }

    fn find_entity_by_id_mut<'a>(&'a mut self, mut id: EntityId) -> Option<EntityMut<'a, 'api>> {
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

    fn collect_entities_mut<'a, F>(
        &'a mut self,
        id: EntityId,
        results: &mut Vec<(EntityId, EntityMut<'a, 'api>)>,
        predicate: F,
    ) where
        'a: 'api,
        F: Fn(&EntityId, &EntityMut<'a, 'api>) -> bool,
    {
        todo!()
    }

    fn collect_types_mut(&mut self, id: EntityId, results: &mut Vec<(EntityId, &mut TypeRef)>) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{Semantics, Type, TypeAlias, TypeRef};

    fn test_alias<'a>() -> TypeAlias<'a> {
        TypeAlias {
            name: "alias",
            target_ty: TypeRef::new(Type::U32, Semantics::Value),
            attributes: Default::default(),
        }
    }

    mod find_entity {
        use crate::model::api::ty_alias::tests::test_alias;
        use crate::model::entity::test_macros::{
            find_entity_match_predicate, find_entity_no_match_predicate,
        };
        use crate::model::entity::FindEntity;
        use crate::model::{Entity, EntityId, EntityType};

        #[test]
        fn match_predicate() {
            let alias = test_alias();
            find_entity_match_predicate!(alias, TypeAlias);
        }

        #[test]
        fn no_match_predicate() {
            let alias = test_alias();
            find_entity_no_match_predicate!(alias, Dto);
        }
    }

    mod find_entity_by_id {
        use crate::model::api::ty_alias::tests::test_alias;
        use crate::model::entity::test_macros::{
            find_entity_by_id_found, find_entity_by_id_not_found,
        };
        use crate::model::entity::FindEntity;
        use crate::model::{Entity, EntityId, EntityType};

        #[test]
        fn found() {
            let alias = test_alias();
            find_entity_by_id_found!(alias, TypeAlias);
        }

        #[test]
        fn not_found() {
            let alias = test_alias();
            find_entity_by_id_not_found!(alias);
        }

        #[test]
        fn found_ty() {
            let alias = test_alias();
            let id = EntityId::try_from("target_ty").unwrap();
            let entity = alias.find_entity_by_id(id).unwrap();
            assert_eq!(entity.ty(), EntityType::Type);
            if let Entity::Type(actual) = entity {
                assert_eq!(&alias.target_ty, actual);
            }
        }
    }

    mod collect_entities {
        use crate::model::api::ty_alias::tests::test_alias;
        use crate::model::entity::test_macros::{
            collect_entities_match_predicate, collect_entities_no_match_predicate,
        };
        use crate::model::entity::FindEntity;
        use crate::model::{Entity, EntityId, EntityType};

        #[test]
        fn match_predicate() {
            let alias = test_alias();
            collect_entities_match_predicate!(alias, TypeAlias);
        }

        #[test]
        fn no_match_predicate() {
            let alias = test_alias();
            collect_entities_no_match_predicate!(alias, Dto);
        }
    }

    mod collect_types {
        use crate::model::api::ty_alias::tests::test_alias;
        use crate::model::entity::FindEntity;
        use crate::model::{EntityId, EntityType};

        #[test]
        fn finds_target_ty() {
            let alias = test_alias();
            let alias_id = EntityId::new_unqualified("something");
            let mut collected = Vec::new();

            alias.collect_types(alias_id.clone(), &mut collected);
            assert!(!collected.is_empty());

            let (id, actual) = collected.first().unwrap();
            assert_eq!(*id, alias_id.child(EntityType::Type, "target_ty").unwrap());
            assert_eq!(*actual, &alias.target_ty);
        }
    }
}
