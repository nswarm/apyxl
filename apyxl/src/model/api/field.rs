use crate::model::attributes::AttributesHolder;
use crate::model::entity::{AsEntity, EntityMut, FindEntity, FindEntityMut};
use crate::model::{entity, Attributes, Entity, EntityId, EntityType, Rpc, TypeRef};

/// A pair of name and type that describe a named instance of a type e.g. within a [Dto] or [Rpc].
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Field<'a> {
    pub name: &'a str,
    pub ty: TypeRef,
    pub attributes: Attributes<'a>,

    /// True if owned by a namespace rather than a Dto.
    /// This member is unused for rpc params. (Yes that's a design flaw).
    pub is_static: bool,
}

impl<'api> AsEntity<'api> for Field<'api> {
    fn as_entity<'a>(&'a self) -> Entity<'a, 'api> {
        Entity::Field(self)
    }

    fn as_entity_mut<'a>(&'a mut self) -> EntityMut<'a, 'api> {
        EntityMut::Field(self)
    }
}

impl AttributesHolder for Field<'_> {
    fn attributes(&self) -> &Attributes {
        &self.attributes
    }
}

impl<'api> FindEntity<'api> for Field<'api> {
    fn find_entity<'a, F>(
        &'a self,
        id: EntityId,
        predicate: F,
    ) -> Option<(EntityId, Entity<'a, 'api>)>
    where
        'a: 'api,
        F: Fn(&EntityId, &Entity<'a, 'api>) -> bool,
    {
        todo!()
    }

    fn find_entity_by_id<'a>(&'a self, mut id: EntityId) -> Option<Entity<'a, 'api>> {
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
                | EntityType::TypeAlias
                | EntityType::Field => None,
            }
        } else {
            Some(Entity::Field(self))
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

impl<'api> FindEntityMut<'api> for Field<'api> {
    fn find_entity_mut<'a, F>(
        &'a mut self,
        id: EntityId,
        predicate: F,
    ) -> Option<(EntityId, EntityMut<'a, 'api>)>
    where
        'a: 'api,
        F: Fn(&EntityId, &EntityMut<'a, 'api>) -> bool,
    {
        todo!()
    }

    fn find_entity_by_id_mut<'a>(&'a mut self, mut id: EntityId) -> Option<EntityMut<'a, 'api>> {
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
                | EntityType::TypeAlias
                | EntityType::Field => None,
            }
        } else {
            Some(EntityMut::Field(self))
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
    use crate::model::{Field, Semantics, Type, TypeRef};

    fn test_field<'a>() -> Field<'a> {
        Field {
            name: "field",
            ty: TypeRef::new(Type::U32, Semantics::Value),
            attributes: Default::default(),
            is_static: false,
        }
    }

    mod find_entity {
        use crate::model::api::field::tests::test_field;
        use crate::model::entity::test_macros::{
            find_entity_match_predicate, find_entity_no_match_predicate,
        };
        use crate::model::entity::FindEntity;
        use crate::model::{Entity, EntityId, EntityType};

        #[test]
        fn match_predicate() {
            let field = test_field();
            find_entity_match_predicate!(field, Field);
        }

        #[test]
        fn no_match_predicate() {
            let field = test_field();
            find_entity_no_match_predicate!(field, Dto);
        }
    }

    mod find_entity_by_id {
        use crate::model::api::field::tests::test_field;
        use crate::model::entity::test_macros::{
            find_entity_by_id_found, find_entity_by_id_not_found,
        };
        use crate::model::entity::FindEntity;
        use crate::model::{Entity, EntityId, EntityType};

        #[test]
        fn found() {
            let field = test_field();
            find_entity_by_id_found!(field, Field);
        }

        #[test]
        fn not_found() {
            let field = test_field();
            find_entity_by_id_not_found!(field);
        }

        #[test]
        fn found_ty() {
            let field = test_field();
            let id = EntityId::try_from("ty").unwrap();
            let entity = field.find_entity_by_id(id).unwrap();
            assert_eq!(entity.ty(), EntityType::Type);
            if let Entity::Type(actual) = entity {
                assert_eq!(field.ty, *actual);
            }
        }
    }

    mod collect_entities {
        use crate::model::api::field::tests::test_field;
        use crate::model::entity::test_macros::{
            collect_entities_match_predicate, collect_entities_no_match_predicate,
        };
        use crate::model::entity::FindEntity;
        use crate::model::{Entity, EntityId, EntityType};

        #[test]
        fn match_predicate() {
            let field = test_field();
            collect_entities_match_predicate!(field, Field);
        }

        #[test]
        fn no_match_predicate() {
            let field = test_field();
            collect_entities_no_match_predicate!(field, Dto);
        }
    }

    mod collect_types {
        use crate::model::api::field::tests::test_field;
        use crate::model::entity::FindEntity;
        use crate::model::{EntityId, EntityType};

        #[test]
        fn finds_ty() {
            let field = test_field();
            let field_id = EntityId::new_unqualified("something");
            let mut collected = Vec::new();

            field.collect_types(field_id.clone(), &mut collected);
            assert!(!collected.is_empty());

            let (id, actual) = collected.first().unwrap();
            assert_eq!(*id, field_id.child(EntityType::Type, "ty").unwrap());
            assert_eq!(*actual, &field.ty);
        }
    }
}
