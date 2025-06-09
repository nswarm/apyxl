use crate::model::api::entity::AsEntity;
use crate::model::attributes::AttributesHolder;
use crate::model::entity::{EntityMut, FindEntity, FindEntityMut};
use crate::model::{Attributes, Entity, EntityId, EntityType, Field, Namespace, Rpc, TypeRef};

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

impl<'api> AsEntity<'api> for Dto<'api> {
    fn as_entity<'a>(&'a self) -> Entity<'a, 'api> {
        Entity::Dto(self)
    }

    fn as_entity_mut<'a>(&'a mut self) -> EntityMut<'a, 'api> {
        EntityMut::Dto(self)
    }
}

impl AttributesHolder for Dto<'_> {
    fn attributes(&self) -> &Attributes {
        &self.attributes
    }
}

impl<'api> FindEntity<'api> for Dto<'api> {
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
                    .and_then(|x| x.find_entity_by_id(id.clone()))
                    .or_else(|| self.namespace.as_ref()?.find_entity_by_id(static_id)),

                EntityType::Rpc => self
                    .rpc(&name)
                    .and_then(|x| x.find_entity_by_id(id.clone()))
                    .or_else(|| self.namespace.as_ref()?.find_entity_by_id(static_id)),

                EntityType::Dto | EntityType::TypeAlias => {
                    self.namespace.as_ref()?.find_entity_by_id(static_id)
                }

                EntityType::None | EntityType::Namespace | EntityType::Enum | EntityType::Type => {
                    None
                }
            }
        } else {
            Some(Entity::Dto(self))
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

impl<'api> FindEntityMut<'api> for Dto<'api> {
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
                            .and_then(|x| x.find_entity_by_id_mut(id.clone()))
                    } else {
                        self.namespace.as_mut()?.find_entity_by_id_mut(static_id)
                    }
                }

                EntityType::Rpc => {
                    let is_member = self.rpc(&name).is_some();
                    if is_member {
                        self.rpc_mut(&name)
                            .and_then(|x| x.find_entity_by_id_mut(id.clone()))
                    } else {
                        self.namespace.as_mut()?.find_entity_by_id_mut(static_id)
                    }
                }

                EntityType::Dto | EntityType::TypeAlias => {
                    self.namespace.as_mut()?.find_entity_by_id_mut(static_id)
                }

                EntityType::None | EntityType::Namespace | EntityType::Enum | EntityType::Type => {
                    None
                }
            }
        } else {
            Some(EntityMut::Dto(self))
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
    use crate::model::{
        Dto, Field, Namespace, NamespaceChild, Rpc, Semantics, Type, TypeAlias, TypeRef,
    };

    fn test_dto<'a>() -> Dto<'a> {
        Dto {
            name: "dto",
            fields: vec![
                Field {
                    name: "field0",
                    ty: TypeRef::new(Type::U32, Semantics::Value),
                    attributes: Default::default(),
                    is_static: false,
                },
                Field {
                    name: "field1",
                    ty: TypeRef::new(Type::U32, Semantics::Value),
                    attributes: Default::default(),
                    is_static: false,
                },
            ],
            rpcs: vec![Rpc {
                name: "rpc",
                params: vec![],
                return_type: None,
                attributes: Default::default(),
                is_static: false,
            }],
            attributes: Default::default(),
            namespace: Some(Namespace {
                name: Default::default(),
                children: vec![NamespaceChild::TypeAlias(TypeAlias {
                    name: "alias",
                    target_ty: TypeRef::new(Type::U32, Semantics::Value),
                    attributes: Default::default(),
                })],
                attributes: Default::default(),
                is_virtual: false,
            }),
        }
    }

    mod find_entity {
        use crate::model::api::dto::tests::test_dto;
        use crate::model::entity::test_macros::{
            find_entity_match_predicate, find_entity_no_match_predicate,
        };
        use crate::model::entity::{AsEntity, FindEntity};
        use crate::model::{Entity, EntityId, EntityType};

        #[test]
        fn match_predicate() {
            let dto = test_dto();
            find_entity_match_predicate!(dto, Dto);
        }

        #[test]
        fn no_match_predicate() {
            let dto = test_dto();
            find_entity_no_match_predicate!(dto, Rpc);
        }

        #[test]
        fn find_field() {
            let dto = test_dto();
            let found = dto.find_entity(EntityId::default(), |_, e| e.ty() == EntityType::Field);
            assert!(found.is_some());

            let (id, entity) = found.unwrap();
            assert_eq!(id, EntityId::try_from("f:field0").unwrap());
            assert_eq!(entity, dto.fields[0].as_entity());
        }

        #[test]
        fn find_rpc() {
            let dto = test_dto();
            let found = dto.find_entity(EntityId::default(), |_, e| e.ty() == EntityType::Rpc);
            assert!(found.is_some());

            let (id, entity) = found.unwrap();
            assert_eq!(id, EntityId::try_from("r:rpc").unwrap());
            assert_eq!(entity, dto.rpcs[0].as_entity());
        }

        #[test]
        fn find_from_namespace() {
            let dto = test_dto();
            let found =
                dto.find_entity(EntityId::default(), |_, e| e.ty() == EntityType::TypeAlias);
            assert!(found.is_some());

            let (id, entity) = found.unwrap();
            assert_eq!(id, EntityId::try_from("a:alias").unwrap());
            let namespace = dto.namespace.as_ref().unwrap();
            assert_eq!(entity, namespace.ty_alias("alias").unwrap().as_entity());
        }
    }

    mod find_entity_by_id {
        use crate::model::api::dto::tests::test_dto;
        use crate::model::entity::test_macros::{
            find_entity_by_id_found, find_entity_by_id_not_found,
        };
        use crate::model::entity::FindEntity;
        use crate::model::{Entity, EntityId, EntityType};

        #[test]
        fn found() {
            let dto = test_dto();
            find_entity_by_id_found!(dto, Dto);
        }

        #[test]
        fn not_found() {
            let dto = test_dto();
            find_entity_by_id_not_found!(dto);
        }

        #[test]
        fn found_field() {
            let dto = test_dto();
            let check_field = |id: EntityId, index: usize| {
                let entity = dto.find_entity_by_id(id).unwrap();
                assert_eq!(entity.ty(), EntityType::Field);
                if let Entity::Field(actual) = entity {
                    assert_eq!(&dto.fields[index], actual);
                }
            };
            check_field(EntityId::try_from("f:field0").unwrap(), 0);
            check_field(EntityId::try_from("f:field1").unwrap(), 1);
        }

        #[test]
        fn found_rpc() {
            let dto = test_dto();
            let id = EntityId::try_from("rpc").unwrap();
            let entity = dto.find_entity_by_id(id).unwrap();
            assert_eq!(entity.ty(), EntityType::Rpc);
            if let Entity::Rpc(actual) = entity {
                assert_eq!(*dto.rpc("rpc").as_ref().unwrap(), actual);
            }
        }

        #[test]
        fn found_namespace_child() {
            let dto = test_dto();
            let id = EntityId::try_from("a:alias").unwrap();
            let entity = dto.find_entity_by_id(id).unwrap();
            assert_eq!(entity.ty(), EntityType::TypeAlias);
            if let Entity::TypeAlias(actual) = entity {
                let namespace = dto.namespace.as_ref().unwrap();
                assert_eq!(namespace.ty_alias("a:alias").unwrap(), actual);
            }
        }
    }

    mod collect_entities {
        use crate::model::api::dto::tests::test_dto;
        use crate::model::entity::test_macros::{
            collect_entities_match_predicate, collect_entities_no_match_predicate,
        };
        use crate::model::entity::{AsEntity, FindEntity};
        use crate::model::{Entity, EntityId, EntityType};

        #[test]
        fn match_predicate() {
            let dto = test_dto();
            collect_entities_match_predicate!(dto, Dto);
        }

        #[test]
        fn no_match_predicate() {
            let dto = test_dto();
            collect_entities_no_match_predicate!(dto, Rpc);
        }

        #[test]
        fn collects_fields() {
            let dto = test_dto();
            let mut collected = Vec::new();
            dto.collect_entities(EntityId::default(), &mut collected, |_, e| {
                e.ty() == EntityType::Field
            });
            assert_eq!(collected.len(), 2);

            let (id0, entity0) = &collected[0];
            assert_eq!(*id0, EntityId::try_from("f:field0").unwrap());
            assert_eq!(*entity0, dto.fields[0].as_entity());

            let (id1, entity1) = &collected[1];
            assert_eq!(*id1, EntityId::try_from("f:field1").unwrap());
            assert_eq!(*entity1, dto.fields[1].as_entity());
        }

        #[test]
        fn collects_rpcs() {
            let dto = test_dto();
            let mut collected = Vec::new();
            dto.collect_entities(EntityId::default(), &mut collected, |_, e| {
                e.ty() == EntityType::Rpc
            });
            assert_eq!(collected.len(), 1);

            let (id, entity) = &collected[0];
            assert_eq!(*id, EntityId::try_from("r:rpc").unwrap());
            assert_eq!(*entity, dto.rpcs[0].as_entity());
        }

        #[test]
        fn collects_from_namespace() {
            let dto = test_dto();
            let mut collected = Vec::new();
            dto.collect_entities(EntityId::default(), &mut collected, |_, e| {
                e.ty() == EntityType::TypeAlias
            });
            assert_eq!(collected.len(), 1);

            let (id, entity) = &collected[0];
            assert_eq!(*id, EntityId::try_from("a:alias").unwrap());
            let namespace = dto.namespace.as_ref().unwrap();
            assert_eq!(*entity, namespace.rpc("rpc").unwrap().as_entity());
        }
    }

    mod collect_types {
        use crate::model::api::dto::tests::test_dto;
        use crate::model::entity::FindEntity;
        use crate::model::{EntityId, EntityType};

        #[test]
        fn finds_field_ty() {
            let dto = test_dto();
            let dto_id = EntityId::new_unqualified("something");
            let mut collected = Vec::new();

            dto.collect_types(dto_id.clone(), &mut collected);
            assert!(!collected.is_empty());

            let (id, actual) = collected.first().unwrap();
            assert_eq!(
                *id,
                dto_id
                    .child(EntityType::Field, "field0")
                    .unwrap()
                    .child(EntityType::Type, "ty")
                    .unwrap()
            );
            assert_eq!(*actual, &dto.field("field0").unwrap().ty);
        }

        #[test]
        fn finds_rpc_ty() {
            let dto = test_dto();
            let dto_id = EntityId::new_unqualified("something");
            let mut collected = Vec::new();

            dto.collect_types(dto_id.clone(), &mut collected);
            assert!(!collected.is_empty());

            let (id, actual) = collected.first().unwrap();
            assert_eq!(
                *id,
                dto_id
                    .child(EntityType::Rpc, "rpc")
                    .unwrap()
                    .child(EntityType::Type, "return_ty")
                    .unwrap()
            );
            let rpc = dto.rpc("rpc").unwrap();
            assert_eq!(*actual, rpc.return_type.as_ref().unwrap());
        }

        #[test]
        fn finds_namespace_child_ty() {
            let dto = test_dto();
            let dto_id = EntityId::new_unqualified("something");
            let mut collected = Vec::new();

            dto.collect_types(dto_id.clone(), &mut collected);
            assert!(!collected.is_empty());

            let (id, actual) = collected.first().unwrap();
            assert_eq!(
                *id,
                dto_id
                    .child(EntityType::TypeAlias, "alias")
                    .unwrap()
                    .child(EntityType::Type, "target_ty")
                    .unwrap()
            );
            let ns = dto.namespace.as_ref().unwrap();
            let alias = ns.ty_alias("alias").unwrap();
            assert_eq!(*actual, &alias.target_ty);
        }
    }
}
