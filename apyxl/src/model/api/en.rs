use crate::model::api::entity::AsEntity;
use crate::model::attributes::AttributesHolder;
use crate::model::entity::{EntityMut, FindEntity, FindEntityMut};
use crate::model::{Attributes, Entity, EntityId, TypeRef};

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

impl<'api> AsEntity<'api> for Enum<'api> {
    fn as_entity<'a>(&'a self) -> Entity<'a, 'api> {
        Entity::Enum(self)
    }

    fn as_entity_mut<'a>(&'a mut self) -> EntityMut<'a, 'api> {
        EntityMut::Enum(self)
    }
}

impl AttributesHolder for Enum<'_> {
    fn attributes(&self) -> &Attributes {
        &self.attributes
    }
}

impl<'api> FindEntity<'api> for Enum<'api> {
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

    fn find_entity_by_id<'a>(&'a self, id: EntityId) -> Option<Entity<'a, 'api>> {
        if id.is_empty() {
            Some(Entity::Enum(self))
        } else {
            None
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

impl<'api> FindEntityMut<'api> for Enum<'api> {
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

    fn find_entity_by_id_mut<'a>(&'a mut self, id: EntityId) -> Option<EntityMut<'a, 'api>> {
        if id.is_empty() {
            Some(EntityMut::Enum(self))
        } else {
            None
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
    use crate::model::{Enum, EnumValue};

    fn test_enum<'a>() -> Enum<'a> {
        Enum {
            name: "en",
            values: vec![
                EnumValue {
                    name: "value0",
                    number: 0,
                    attributes: Default::default(),
                },
                EnumValue {
                    name: "value1",
                    number: 1,
                    attributes: Default::default(),
                },
            ],
            attributes: Default::default(),
        }
    }

    mod find_entity {
        use crate::model::api::en::tests::test_enum;
        use crate::model::entity::test_macros::{
            find_entity_match_predicate, find_entity_no_match_predicate,
        };
        use crate::model::entity::FindEntity;
        use crate::model::{Entity, EntityId, EntityType};

        #[test]
        fn match_predicate() {
            let en = test_enum();
            find_entity_match_predicate!(en, Enum);
        }

        #[test]
        fn no_match_predicate() {
            let en = test_enum();
            find_entity_no_match_predicate!(en, Dto);
        }
    }

    mod find_entity_by_id {
        use crate::model::api::en::tests::test_enum;
        use crate::model::entity::test_macros::{
            find_entity_by_id_found, find_entity_by_id_not_found,
        };
        use crate::model::entity::FindEntity;
        use crate::model::{Entity, EntityId, EntityType};

        #[test]
        fn found() {
            let en = test_enum();
            find_entity_by_id_found!(en, Enum);
        }

        #[test]
        fn not_found() {
            let en = test_enum();
            find_entity_by_id_not_found!(en);
        }
    }

    mod collect_entities {
        use crate::model::api::en::tests::test_enum;
        use crate::model::entity::test_macros::{
            collect_entities_match_predicate, collect_entities_no_match_predicate,
        };
        use crate::model::entity::FindEntity;
        use crate::model::{Entity, EntityId, EntityType};

        #[test]
        fn match_predicate() {
            let en = test_enum();
            collect_entities_match_predicate!(en, Enum);
        }

        #[test]
        fn no_match_predicate() {
            let en = test_enum();
            collect_entities_no_match_predicate!(en, Dto);
        }
    }

    mod collect_types {
        use crate::model::api::en::tests::test_enum;
        use crate::model::entity::FindEntity;
        use crate::model::EntityId;

        #[test]
        fn finds_no_types() {
            let en = test_enum();
            let en_id = EntityId::new_unqualified("something");
            let mut collected = Vec::new();

            en.collect_types(en_id.clone(), &mut collected);
            assert!(collected.is_empty());
        }
    }
}
