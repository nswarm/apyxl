use crate::model::api::entity::AsEntity;
use crate::model::attributes::AttributesHolder;
use crate::model::entity::{EntityMut, FindEntity, FindEntityMut};
use crate::model::{entity, Attributes, Entity, EntityId, EntityType, Field, TypeRef};

/// A single Remote Procedure Call (RPC) within an [Api].
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct Rpc<'a> {
    pub name: &'a str,
    pub params: Vec<Field<'a>>,
    pub return_type: Option<TypeRef>,
    pub attributes: Attributes<'a>,

    /// True if owned by a namespace rather than a Dto.
    pub is_static: bool,
}

impl<'a> Rpc<'a> {
    pub fn param(&self, name: &str) -> Option<&Field<'a>> {
        self.params.iter().find(|param| param.name == name)
    }

    pub fn param_mut(&mut self, name: &str) -> Option<&mut Field<'a>> {
        self.params.iter_mut().find(|param| param.name == name)
    }
}

impl<'api> AsEntity<'api> for Rpc<'api> {
    fn as_entity<'a>(&'a self) -> Entity<'a, 'api> {
        Entity::Rpc(self)
    }

    fn as_entity_mut<'a>(&'a mut self) -> EntityMut<'a, 'api> {
        EntityMut::Rpc(self)
    }
}

impl AttributesHolder for Rpc<'_> {
    fn attributes(&self) -> &Attributes {
        &self.attributes
    }
}

impl<'api> FindEntity<'api> for Rpc<'api> {
    fn find_entity_by_id<'a>(&'a self, mut id: EntityId) -> Option<Entity<'a, 'api>> {
        if let Some((ty, name)) = id.pop_front() {
            match ty {
                EntityType::Field => self.param(&name).map_or(None, |x| x.find_entity_by_id(id)),

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
                | EntityType::TypeAlias
                | EntityType::Enum => None,
            }
        } else {
            Some(Entity::Rpc(self))
        }
    }

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

impl<'api> FindEntityMut<'api> for Rpc<'api> {
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
                EntityType::Field => self
                    .param_mut(&name)
                    .and_then(|x| x.find_entity_by_id_mut(id)),

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
                | EntityType::TypeAlias
                | EntityType::Enum => None,
            }
        } else {
            Some(EntityMut::Rpc(self))
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
    use crate::model::{Field, Rpc, Semantics, Type, TypeRef};

    fn test_rpc<'a>() -> Rpc<'a> {
        Rpc {
            name: "rpc",
            params: vec![
                Field {
                    name: "param0",
                    ty: TypeRef::new(Type::U32, Semantics::Value),
                    attributes: Default::default(),
                    is_static: false,
                },
                Field {
                    name: "param1",
                    ty: TypeRef::new(Type::U32, Semantics::Value),
                    attributes: Default::default(),
                    is_static: false,
                },
            ],
            return_type: Some(TypeRef::new(Type::U32, Semantics::Value)),
            attributes: Default::default(),
            is_static: false,
        }
    }

    mod find_entity {
        use crate::model::api::rpc::tests::test_rpc;
        use crate::model::entity::test_macros::{
            find_entity_match_predicate, find_entity_no_match_predicate,
        };
        use crate::model::entity::FindEntity;
        use crate::model::{Entity, EntityId, EntityType};

        #[test]
        fn match_predicate() {
            let rpc = test_rpc();
            find_entity_match_predicate!(rpc, Rpc);
        }

        #[test]
        fn no_match_predicate() {
            let rpc = test_rpc();
            find_entity_no_match_predicate!(rpc, Dto);
        }
    }

    mod find_entity_by_id {
        use crate::model::api::rpc::tests::test_rpc;
        use crate::model::entity::test_macros::{
            find_entity_by_id_found, find_entity_by_id_not_found,
        };
        use crate::model::entity::FindEntity;
        use crate::model::{Entity, EntityId, EntityType};

        #[test]
        fn found() {
            let rpc = test_rpc();
            find_entity_by_id_found!(rpc, Rpc);
        }

        #[test]
        fn not_found() {
            let rpc = test_rpc();
            find_entity_by_id_not_found!(rpc);
        }

        #[test]
        fn found_param() {
            let rpc = test_rpc();
            let check_param = |id: EntityId, index: usize| {
                let entity = rpc.find_entity_by_id(id).unwrap();
                assert_eq!(entity.ty(), EntityType::Field);
                if let Entity::Field(actual) = entity {
                    assert_eq!(&rpc.params[index], actual);
                }
            };
            check_param(EntityId::try_from("p:param0").unwrap(), 0);
            check_param(EntityId::try_from("p:param1").unwrap(), 1);
        }

        #[test]
        fn found_return_ty() {
            let rpc = test_rpc();
            let id = EntityId::try_from("return_ty").unwrap();
            let entity = rpc.find_entity_by_id(id).unwrap();
            assert_eq!(entity.ty(), EntityType::Type);
            if let Entity::Type(actual) = entity {
                assert_eq!(rpc.return_type.as_ref().unwrap(), actual);
            }
        }
    }

    mod collect_entities {
        use crate::model::api::rpc::tests::test_rpc;
        use crate::model::entity::test_macros::{
            collect_entities_match_predicate, collect_entities_no_match_predicate,
        };
        use crate::model::entity::FindEntity;
        use crate::model::{Entity, EntityId, EntityType};

        #[test]
        fn match_predicate() {
            let rpc = test_rpc();
            collect_entities_match_predicate!(rpc, Rpc);
        }

        #[test]
        fn no_match_predicate() {
            let rpc = test_rpc();
            collect_entities_no_match_predicate!(rpc, Dto);
        }
    }

    mod collect_types {
        use crate::model::api::rpc::tests::test_rpc;
        use crate::model::entity::FindEntity;
        use crate::model::{EntityId, EntityType};

        #[test]
        fn finds_return_ty() {
            let rpc = test_rpc();
            let rpc_id = EntityId::new_unqualified("something");
            let mut collected = Vec::new();

            rpc.collect_types(rpc_id.clone(), &mut collected);
            assert!(!collected.is_empty());

            let (id, actual) = collected.first().unwrap();
            assert_eq!(*id, rpc_id.child(EntityType::Type, "return_ty").unwrap());
            assert_eq!(*actual, rpc.return_type.as_ref().unwrap());
        }
    }
}
