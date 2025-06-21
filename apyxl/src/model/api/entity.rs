use anyhow::{anyhow, Result};

use crate::model::{Dto, EntityId, Enum, Field, Namespace, Rpc, TypeAlias, TypeRef};

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Copy, Clone)]
pub enum EntityType {
    None, // Unqualified EntityIds.
    Namespace,
    Dto,
    Rpc,
    Enum,
    Field,
    TypeAlias,
    Type,
}

/// Reference to a specific entity within an API.
pub enum Entity<'a, 'api> {
    Namespace(&'a Namespace<'api>),
    Dto(&'a Dto<'api>),
    Rpc(&'a Rpc<'api>),
    Enum(&'a Enum<'api>),
    Field(&'a Field<'api>),
    TypeAlias(&'a TypeAlias<'api>),
    Type(&'a TypeRef),
}

/// Mutable reference to a specific entity within an API.
pub enum EntityMut<'a, 'api> {
    Namespace(&'a mut Namespace<'api>),
    Dto(&'a mut Dto<'api>),
    Rpc(&'a mut Rpc<'api>),
    Enum(&'a mut Enum<'api>),
    Field(&'a mut Field<'api>),
    TypeAlias(&'a mut TypeAlias<'api>),
    Type(&'a mut TypeRef),
}

pub trait FindEntity<'api> {
    /// Qualify an [EntityId] relative to this [Entity], if possible.
    /// If `referenceable` is true, will only find [Entity]s able to be referenced by a type ref.
    fn qualify_id(&self, id: EntityId, referenceable: bool) -> Result<EntityId>;

    /// Find an [Entity] by qualified [EntityId], if it exists.
    fn find_entity<'a>(&'a self, id: EntityId) -> Option<Entity<'a, 'api>>;

    /// Find an [Entity] by qualified [EntityId], if it exists.
    fn find_entity_mut<'a>(&'a mut self, id: EntityId) -> Option<EntityMut<'a, 'api>>;
}

impl<'api> FindEntity<'api> for Entity<'_, 'api> {
    fn qualify_id(&self, id: EntityId, referenceable: bool) -> Result<EntityId> {
        match self {
            Entity::Namespace(ns) => ns.qualify_id(id, referenceable),
            Entity::Dto(dto) => dto.qualify_id(id, referenceable),
            Entity::Rpc(rpc) => rpc.qualify_id(id, referenceable),
            Entity::Enum(en) => en.qualify_id(id, referenceable),
            Entity::Field(field) => field.qualify_id(id, referenceable),
            Entity::TypeAlias(alias) => alias.qualify_id(id, referenceable),
            Entity::Type(ty) => ty.qualify_id(id, referenceable),
        }
    }

    fn find_entity<'a>(&'a self, id: EntityId) -> Option<Entity<'a, 'api>> {
        match self {
            Entity::Namespace(ns) => ns.find_entity(id),
            Entity::Dto(dto) => dto.find_entity(id),
            Entity::Rpc(rpc) => rpc.find_entity(id),
            Entity::Enum(en) => en.find_entity(id),
            Entity::Field(field) => field.find_entity(id),
            Entity::TypeAlias(alias) => alias.find_entity(id),
            Entity::Type(ty) => ty.find_entity(id),
        }
    }

    fn find_entity_mut<'a>(&'a mut self, _: EntityId) -> Option<EntityMut<'a, 'api>> {
        panic!("cannot find mut through immutable Entity reference")
    }
}

impl<'api> FindEntity<'api> for EntityMut<'_, 'api> {
    fn qualify_id(&self, id: EntityId, referenceable: bool) -> Result<EntityId> {
        match self {
            EntityMut::Namespace(ns) => ns.qualify_id(id, referenceable),
            EntityMut::Dto(dto) => dto.qualify_id(id, referenceable),
            EntityMut::Rpc(rpc) => rpc.qualify_id(id, referenceable),
            EntityMut::Enum(en) => en.qualify_id(id, referenceable),
            EntityMut::Field(field) => field.qualify_id(id, referenceable),
            EntityMut::TypeAlias(alias) => alias.qualify_id(id, referenceable),
            EntityMut::Type(ty) => ty.qualify_id(id, referenceable),
        }
    }

    fn find_entity<'a>(&'a self, id: EntityId) -> Option<Entity<'a, 'api>> {
        match self {
            EntityMut::Namespace(ns) => ns.find_entity(id),
            EntityMut::Dto(dto) => dto.find_entity(id),
            EntityMut::Rpc(rpc) => rpc.find_entity(id),
            EntityMut::Enum(en) => en.find_entity(id),
            EntityMut::Field(field) => field.find_entity(id),
            EntityMut::TypeAlias(alias) => alias.find_entity(id),
            EntityMut::Type(ty) => ty.find_entity(id),
        }
    }

    fn find_entity_mut<'a>(&'a mut self, id: EntityId) -> Option<EntityMut<'a, 'api>> {
        match self {
            EntityMut::Namespace(ns) => ns.find_entity_mut(id),
            EntityMut::Dto(dto) => dto.find_entity_mut(id),
            EntityMut::Rpc(rpc) => rpc.find_entity_mut(id),
            EntityMut::Enum(en) => en.find_entity_mut(id),
            EntityMut::Field(field) => field.find_entity_mut(id),
            EntityMut::TypeAlias(alias) => alias.find_entity_mut(id),
            EntityMut::Type(ty) => ty.find_entity_mut(id),
        }
    }
}

pub trait ToEntity {
    /// Create an [Entity] reference to this entity.
    fn to_entity(&self) -> Entity;

    fn entity_type(&self) -> EntityType {
        self.to_entity().ty()
    }
}

// Keep these in line with [EntityId] documentation.
#[rustfmt::skip]
pub mod subtype {
    pub const NAMESPACE: &str =             "namespace";
    pub const NAMESPACE_SHORTISH: &str =    "ns";
    pub const NAMESPACE_SHORT: &str =       "n";
    pub const DTO: &str =                   "dto";
    pub const DTO_SHORT: &str =             "d";
    pub const RPC: &str =                   "rpc";
    pub const RPC_SHORT: &str =             "r";
    pub const ENUM: &str =                  "enum";
    pub const ENUM_MED: &str =              "en";
    pub const ENUM_SHORT: &str =            "e";
    pub const FIELD: &str =                 "field";
    pub const FIELD_SHORT: &str =           "f";
    pub const PARAM: &str =                 "param";
    pub const PARAM_SHORT: &str =           "p";
    pub const TY: &str =                    "ty";
    pub const RETURN_TY: &str =             "return_ty";
    pub const TY_ALIAS: &str =              "alias";
    pub const TY_ALIAS_SHORT: &str =        "a";
    pub const TY_ALIAS_TARGET: &str =       "target_ty";

    pub const NAMESPACE_ALL: &[&str] = &[NAMESPACE, NAMESPACE_SHORTISH, NAMESPACE_SHORT];
    pub const DTO_ALL: &[&str] = &[DTO, DTO_SHORT];
    pub const RPC_ALL: &[&str] = &[RPC, RPC_SHORT];
    pub const ENUM_ALL: &[&str] = &[ENUM, ENUM_MED, ENUM_SHORT];
    pub const FIELD_ALL: &[&str] = &[FIELD, FIELD_SHORT];
    pub const PARAM_ALL: &[&str] = &[PARAM, PARAM_SHORT];
    pub const TY_ALL: &[&str] = &[TY];
    pub const RETURN_TY_ALL: &[&str] = &[RETURN_TY];
    pub const TY_ALIAS_ALL: &[&str] = &[TY_ALIAS, TY_ALIAS_SHORT];
    pub const TY_ALIAS_TARGET_ALL: &[&str] = &[TY_ALIAS_TARGET];
}

impl EntityType {
    pub fn is_valid_subtype(&self, ty: &EntityType) -> bool {
        // Keep these in line with [EntityId] documentation.
        // All variants are specified so additions to enum will force consideration here.
        match self {
            EntityType::None => *ty == EntityType::None,

            EntityType::Namespace => match ty {
                EntityType::Namespace
                | EntityType::Dto
                | EntityType::Rpc
                | EntityType::Enum
                | EntityType::TypeAlias
                | EntityType::Field => true,
                EntityType::Type | EntityType::None => false,
            },

            EntityType::Dto => match ty {
                EntityType::Field
                | EntityType::Dto
                | EntityType::Rpc
                | EntityType::TypeAlias
                | EntityType::Enum => true,

                EntityType::Namespace | EntityType::Type | EntityType::None => false,
            },

            EntityType::Rpc => match ty {
                EntityType::Field | EntityType::Type => true,
                EntityType::Namespace
                | EntityType::Dto
                | EntityType::Rpc
                | EntityType::Enum
                | EntityType::TypeAlias
                | EntityType::None => false,
            },

            EntityType::Enum => match ty {
                EntityType::Namespace
                | EntityType::Dto
                | EntityType::Rpc
                | EntityType::Enum
                | EntityType::Type
                | EntityType::TypeAlias
                | EntityType::Field
                | EntityType::None => false,
            },

            EntityType::Field => match ty {
                EntityType::Type => true,
                EntityType::Namespace
                | EntityType::Dto
                | EntityType::Rpc
                | EntityType::Enum
                | EntityType::TypeAlias
                | EntityType::Field
                | EntityType::None => false,
            },

            EntityType::Type => match ty {
                EntityType::Namespace
                | EntityType::Dto
                | EntityType::Rpc
                | EntityType::Enum
                | EntityType::Type
                | EntityType::TypeAlias
                | EntityType::Field
                | EntityType::None => false,
            },

            EntityType::TypeAlias => match ty {
                EntityType::Type => true,
                EntityType::Namespace
                | EntityType::Dto
                | EntityType::Rpc
                | EntityType::Enum
                | EntityType::TypeAlias
                | EntityType::Field
                | EntityType::None => false,
            },
        }
    }
}

impl Entity<'_, '_> {
    pub fn ty(&self) -> EntityType {
        match self {
            Entity::Namespace(ns) => {
                if ns.is_virtual {
                    EntityType::Dto
                } else {
                    EntityType::Namespace
                }
            }
            Entity::Dto(_) => EntityType::Dto,
            Entity::Rpc(_) => EntityType::Rpc,
            Entity::Enum(_) => EntityType::Enum,
            Entity::Field(_) => EntityType::Field,
            Entity::Type(_) => EntityType::Type,
            Entity::TypeAlias(_) => EntityType::TypeAlias,
        }
    }
}

impl TryFrom<&str> for EntityType {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            _ if subtype::NAMESPACE_ALL.contains(&value) => Ok(EntityType::Namespace),
            _ if subtype::DTO_ALL.contains(&value) => Ok(EntityType::Dto),
            _ if subtype::RPC_ALL.contains(&value) => Ok(EntityType::Rpc),
            _ if subtype::ENUM_ALL.contains(&value) => Ok(EntityType::Enum),
            _ if subtype::FIELD_ALL.contains(&value) => Ok(EntityType::Field),
            _ if subtype::PARAM_ALL.contains(&value) => Ok(EntityType::Field),
            _ if subtype::TY_ALL.contains(&value) => Ok(EntityType::Type),
            _ if subtype::RETURN_TY_ALL.contains(&value) => Ok(EntityType::Type),
            _ if subtype::TY_ALIAS_TARGET_ALL.contains(&value) => Ok(EntityType::Type),
            _ if subtype::TY_ALIAS_ALL.contains(&value) => Ok(EntityType::TypeAlias),
            _ => Err(anyhow!(
                "subtype '{}' does not map to a valid EntityType",
                value
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    mod qualify_id {
        use crate::model::entity::FindEntity;
        use crate::model::EntityId;
        use crate::test_util::executor::TestExecutor;
        use anyhow::Result;

        #[test]
        fn dto() -> Result<()> {
            let input = r#"
            mod ns {
                struct dto {}
            }
            "#;
            run_test(input, true, "ns.dto", "ns:ns.d:dto")?;
            Ok(())
        }

        #[test]
        fn dto_field() -> Result<()> {
            let input = r#"
            mod ns {
                struct dto {
                    field: i32
                }
            }
            "#;
            run_test(input, false, "ns.dto.field", "ns:ns.d:dto.f:field")
        }

        #[test]
        fn dto_field_ty() -> Result<()> {
            let input = r#"
            mod ns {
                struct dto {
                    field: i32
                }
            }
            "#;
            run_test(input, false, "ns.dto.field.ty", "ns:ns.d:dto.f:field.ty")
        }

        // todo pyx - nested rpc in addition to virtual ns

        #[test]
        fn dto_rpc() -> Result<()> {
            let input = r#"
            mod ns {
                struct dto {}
                impl dto {
                    fn rpc() {}
                }
            }
            "#;
            run_test(input, false, "ns.dto.rpc", "ns:ns.d:dto.r:rpc")
        }

        #[test]
        fn dto_rpc_param() -> Result<()> {
            let input = r#"
            mod ns {
                struct dto {}
                impl dto {
                    fn rpc(param: i32) {}
                }
            }
            "#;
            run_test(
                input,
                false,
                "ns.dto.rpc.param",
                "ns:ns.d:dto.r:rpc.p:param",
            )
        }

        #[test]
        fn dto_rpc_param_ty() -> Result<()> {
            let input = r#"
            mod ns {
                struct dto {}
                impl dto {
                    fn rpc(param: i32) {}
                }
            }
            "#;
            run_test(
                input,
                false,
                "ns.dto.rpc.param.ty",
                "ns:ns.d:dto.r:rpc.p:param.ty",
            )
        }

        #[test]
        fn dto_rpc_return_ty() -> Result<()> {
            let input = r#"
            mod ns {
                struct dto {}
                impl dto {
                    fn rpc() -> i32 {}
                }
            }
            "#;
            run_test(
                input,
                false,
                "ns.dto.rpc.return_ty",
                "ns:ns.d:dto.r:rpc.return_ty",
            )
        }

        // todo pyx - support nested
        // #[test]
        // fn dto_en() -> Result<()> {
        //     let input = r#"
        //     mod ns {
        //         struct dto {
        //             enum en {}
        //         }
        //     }
        //     "#;
        //     run_test(input, true, "ns.dto.en", "ns:ns.d:dto.e:en")
        // }

        // todo pyx - support nested
        // #[test]
        // fn dto_nested() -> Result<()> {
        //     let input = r#"
        //     mod ns {
        //         struct dto {
        //             struct dto2 {}
        //         }
        //     }
        //     "#;
        //     run_test(input, true, "ns.dto.dto2", "ns:ns.d:dto.d:dto2")
        // }

        // todo pyx - support nested
        // #[test]
        // fn dto_deeply_nested() -> Result<()> {
        //     let input = r#"
        //     mod ns {
        //         struct dto {
        //             struct dto2 {
        //                 struct dto3 {}
        //             }
        //         }
        //     }
        //     "#;
        //     run_test(input, true, "ns.dto.dto2.dto3", "ns:ns.d:dto.d:dto2.d:dto3")
        // }

        #[test]
        fn field() -> Result<()> {
            let input = r#"
            mod ns {
                const field: i32 = 5;
            }
            "#;
            run_test(input, false, "ns.field", "ns:ns.f:field")
        }

        #[test]
        fn field_ty() -> Result<()> {
            let input = r#"
            mod ns {
                const field: i32 = 5;
            }
            "#;
            run_test(input, false, "ns.field.ty", "ns:ns.f:field.ty")
        }

        #[test]
        fn rpc() -> Result<()> {
            let input = r#"
            mod ns {
                fn rpc() {}
            }
            "#;
            run_test(input, false, "ns.rpc", "ns:ns.r:rpc")
        }

        #[test]
        fn rpc_param() -> Result<()> {
            let input = r#"
            mod ns {
                fn rpc(param: i32) {}
            }
            "#;
            run_test(input, false, "ns.rpc.param", "ns:ns.r:rpc.p:param")
        }

        #[test]
        fn rpc_param_ty() -> Result<()> {
            let input = r#"
            mod ns {
                fn rpc(param: i32) {}
            }
            "#;
            run_test(input, false, "ns.rpc.param.ty", "ns:ns.r:rpc.p:param.ty")
        }

        #[test]
        fn rpc_return_ty() -> Result<()> {
            let input = r#"
            mod ns {
                fn rpc() -> i32 {}
            }
            "#;
            run_test(input, false, "ns.rpc.return_ty", "ns:ns.r:rpc.return_ty")
        }

        #[test]
        fn en() -> Result<()> {
            let input = r#"
            mod ns {
                enum en {}
            }
            "#;
            run_test(input, true, "ns.en", "ns:ns.e:en")
        }

        #[test]
        fn type_alias() -> Result<()> {
            let input = r#"
            mod ns {
                type alias = i32;
            }
            "#;
            run_test(input, true, "ns.alias", "ns:ns.a:alias")
        }

        #[test]
        fn type_alias_target_ty() -> Result<()> {
            let input = r#"
            mod ns {
                type alias = i32;
            }
            "#;
            run_test(
                input,
                false,
                "ns.alias.target_ty",
                "ns:ns.a:alias.target_ty",
            )
        }

        #[test]
        fn namespace() -> Result<()> {
            let input = r#"
            mod ns {}
            "#;
            run_test(input, true, "ns", "ns:ns")
        }

        #[test]
        fn namespace_nested() -> Result<()> {
            let input = r#"
            mod ns {
                mod ns2 {}
            }
            "#;
            run_test(input, true, "ns.ns2", "ns:ns.ns:ns2")
        }

        #[test]
        fn namespace_deeply_nested() -> Result<()> {
            let input = r#"
            mod ns {
                mod ns2 {
                    mod ns3 {}
                }
            }
            "#;
            run_test(input, true, "ns.ns2.ns3", "ns:ns.ns:ns2.ns:ns3")
        }

        fn run_test(
            input: &str,
            is_entity_referenceable: bool,
            qualify_id: &str,
            expected_id: &str,
        ) -> Result<()> {
            let result = test_qualify_id(input, true, qualify_id, expected_id);
            if is_entity_referenceable {
                result?;
            } else {
                // If type is NOT referenceable, it should result in error when trying to qualify.
                assert!(result.is_err());
            }
            test_qualify_id(input, false, qualify_id, expected_id)
        }

        fn test_qualify_id(
            input: &str,
            referenceable: bool,
            qualify_id: &str,
            expected_id: &str,
        ) -> Result<()> {
            let mut exe = TestExecutor::new(input);
            let api = exe.api();
            let actual = api.qualify_id(EntityId::new_unqualified(qualify_id), referenceable)?;
            assert_eq!(
                actual,
                EntityId::try_from(expected_id)?,
                "qualify_id: {} (referenceable: {})",
                qualify_id,
                referenceable
            );
            Ok(())
        }
    }
}
