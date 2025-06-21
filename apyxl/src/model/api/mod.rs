pub use attributes::Attributes;
pub use attributes::Comment;
pub use dependencies::Dependencies;
pub use dto::Dto;
pub use en::Enum;
pub use en::EnumValue;
pub use en::EnumValueNumber;
pub use entity::Entity;
pub use entity::EntityType;
pub use entity_id::EntityId;
pub use field::Field;
pub use namespace::Namespace;
pub use namespace::NamespaceChild;
pub use rpc::Rpc;
pub use ty::BaseType;
pub use ty::Semantics;
pub use ty::Type;
pub use ty::TypeRef;
pub use ty::UserTypeName;
pub use ty_alias::TypeAlias;
pub use validate::ValidationError;

pub mod attributes;
mod dependencies;
mod dto;
mod en;
pub mod entity;
mod entity_id;
mod field;
mod namespace;
mod rpc;
mod ty;
mod ty_alias;
pub mod validate;

use crate::model::entity::FindEntity;
use anyhow::{anyhow, Result};

/// The root namespace of the entire API.
pub const UNDEFINED_NAMESPACE: &str = "_";

/// A complete set of entities that make up an API.
pub type Api<'a> = Namespace<'a>;

impl Api<'_> {
    /// Find `find_ty` by walking up the namespace hierarchy, starting at `initial_namespace`.
    /// Returns the fully qualified type [EntityId] or errors if it does not exist.
    /// Only supports finding referenceable entity types.
    pub fn find_qualified_type_relative(
        &self,
        initial_namespace: &EntityId,
        find_ty: &EntityId,
    ) -> Result<EntityId> {
        // We're going to qualify anyway, so avoid errors with mixed ids.
        let mut iter = initial_namespace.to_unqualified();
        loop {
            if let Ok(qualified) = self.qualify_id(iter.concat(find_ty)?, true) {
                return Ok(qualified);
            }
            iter = iter.parent().ok_or(anyhow!(
                "failed to qualify id {} starting in namespace {}",
                find_ty,
                initial_namespace
            ))?;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::model::EntityId;
    use crate::test_util::executor::TestExecutor;
    use anyhow::Result;

    #[test]
    fn dto_from_root() -> Result<()> {
        let initial_namespace = EntityId::default();
        let find_id = EntityId::new_unqualified("ns0.dto");
        run_test(
            r#"
            mod ns0 {
                struct dto {}
            }
            "#,
            &initial_namespace,
            &find_id,
            EntityId::try_from("ns0.d:dto")?,
        )
    }

    #[test]
    fn dto_from_ns() -> Result<()> {
        let initial_namespace = EntityId::new_unqualified("ns0");
        let find_id = EntityId::new_unqualified("ns1.dto");
        run_test(
            r#"
            mod ns0 {
                mod ns1 {
                    struct dto {}
                }
            }
            "#,
            &initial_namespace,
            &find_id,
            EntityId::try_from("ns0.ns1.d:dto").unwrap(),
        )
    }

    #[test]
    fn dto_from_sibling() -> Result<()> {
        let initial_namespace = EntityId::new_unqualified("ns0.other");
        let find_id = EntityId::new_unqualified("ns1.dto");
        run_test(
            r#"
            mod ns0 {
                mod ns1 {
                    struct dto {}
                }
                mod other {
                }
            }
            "#,
            &initial_namespace,
            &find_id,
            EntityId::try_from("ns0.ns1.d:dto").unwrap(),
        )
    }

    #[test]
    fn dto_overqualified() -> Result<()> {
        let initial_namespace = EntityId::new_unqualified("ns0.ns1.ns2.other");
        let find_id = EntityId::new_unqualified("ns1.ns2.dto");
        run_test(
            r#"
            mod ns0 {
                mod ns1 {
                    mod ns2 {
                        struct dto {}
                        mod other {}
                    }
                }
            }
            "#,
            &initial_namespace,
            &find_id,
            EntityId::try_from("ns0.ns1.ns2.d:dto").unwrap(),
        )
    }

    #[test]
    fn enum_from_ns() -> Result<()> {
        let initial_namespace = EntityId::new_unqualified("ns0");
        let find_id = EntityId::new_unqualified("ns1.en");
        run_test(
            r#"
            mod ns0 {
                mod ns1 {
                    enum en {}
                }
            }
            "#,
            &initial_namespace,
            &find_id,
            EntityId::try_from("ns0.ns1.e:en").unwrap(),
        )
    }

    #[test]
    fn ty_alias_from_ns() -> Result<()> {
        let initial_namespace = EntityId::new_unqualified("ns0");
        let find_id = EntityId::new_unqualified("ns1.alias");
        run_test(
            r#"
            mod ns0 {
                mod ns1 {
                    type alias = u32;
                }
            }
            "#,
            &initial_namespace,
            &find_id,
            EntityId::try_from("ns0.ns1.a:alias").unwrap(),
        )
    }

    #[test]
    fn field_from_ns() -> Result<()> {
        let initial_namespace = EntityId::new_unqualified("ns0");
        let find_id = EntityId::new_unqualified("ns1.field");
        run_test(
            r#"
            mod ns0 {
                mod ns1 {
                    const field: u32 = 5;
                }
            }
            "#,
            &initial_namespace,
            &find_id,
            EntityId::try_from("ns0.ns1.f:field").unwrap(),
        )
    }

    // todo can't use rust parser for tests of things like enums/dtos nested inside dtos :\
    // todo ensure correct qualification type i.e. uses dto if necessary
    // also need to test multiple-depth dto nesting to ensure we get dto:x.dto:y.dto:z etc.
    // #[test]
    // fn x_inside_dto() -> Result<()> {
    //     let initial_namespace = EntityId::new_unqualified("ns0");
    //     let find_id = EntityId::new_unqualified("ns1.field");
    //     run_test(
    //         r#"
    //         mod ns0 {
    //             const field: u32 = 5;
    //         }
    //         "#,
    //         &initial_namespace,
    //         &find_id,
    //         EntityId::try_from("ns0.ns1.f:field").unwrap(),
    //     )
    // }

    #[test]
    fn does_not_exist() {
        let initial_namespace = EntityId::default();
        let find_id = EntityId::new_unqualified("asdf.dto");
        let result = run_test(
            r#"
            mod ns0 {
                struct dto {}
            }
            "#,
            &initial_namespace,
            &find_id,
            EntityId::default(),
        );
        assert!(result.is_err());
    }

    fn run_test(
        data: &str,
        initial_namespace: &EntityId,
        find_ty: &EntityId,
        expected: EntityId,
    ) -> Result<()> {
        let mut exe = TestExecutor::new(data);
        let api = exe.api();
        assert_eq!(
            api.find_qualified_type_relative(initial_namespace, find_ty)?,
            expected,
        );
        Ok(())
    }
}
