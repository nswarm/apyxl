use anyhow::Result;

use crate::model::{Api, EntityId, TypeRef};

/// Used to separate validation of the api from mutation. Validation functions can return a
/// [Mutation] if during validation they want to make a change.
///
/// In particular, this method gets around a recurse_api_mut, which doesn't work because it
/// requires multiple Api references.
#[derive(Debug)]
pub enum Mutation {
    QualifyType(qualify_type::Data),
}

impl Mutation {
    pub fn new_qualify_type(entity_id: EntityId, new_ty: TypeRef) -> Self {
        Mutation::QualifyType(qualify_type::Data { entity_id, new_ty })
    }

    pub fn execute(self, api: &mut Api) -> Result<()> {
        match self {
            Mutation::QualifyType(data) => qualify_type::execute(api, data)?,
        }
        Ok(())
    }
}

pub mod qualify_type {
    use anyhow::{anyhow, Result};

    use crate::model::entity::{EntityMut, FindEntity};
    use crate::model::{Api, EntityId, TypeRef};

    #[derive(Debug)]
    pub struct Data {
        pub entity_id: EntityId,
        pub new_ty: TypeRef,
    }

    pub fn execute(api: &mut Api, data: Data) -> Result<()> {
        match api.find_entity_mut(data.entity_id.clone()) {
            None => Err(error_entity_id_not_found(&data.entity_id)),
            Some(entity) => match entity {
                EntityMut::Type(ty) => {
                    *ty = data.new_ty;
                    Ok(())
                }
                _ => Err(error_entity_id_is_not_type(&data.entity_id)),
            },
        }
    }

    fn error_entity_id_not_found(entity_id: &EntityId) -> anyhow::Error {
        anyhow!(
            "Mutation::QualifyType failed: Could not find EntityId '{}' in the API",
            entity_id
        )
    }

    fn error_entity_id_is_not_type(entity_id: &EntityId) -> anyhow::Error {
        anyhow!(
            "Mutation::QualifyType failed: EntityId '{}' exists, but is not a Type.",
            entity_id
        )
    }
}

#[cfg(test)]
mod tests {
    mod qualify_type {
        use anyhow::Result;

        use crate::model::validate::mutation::qualify_type;
        use crate::model::validate::Mutation;
        use crate::model::{EntityId, Semantics, Type, TypeRef};
        use crate::test_util::executor::TestExecutor;

        #[test]
        fn success_changes_type() {
            let data = r#"
            struct dto {
                field: SomeType
            }
            "#;
            let new_ty = TypeRef::new(Type::Bool, Semantics::Value);
            let mut exe = TestExecutor::new(data);
            let mut api = exe.api();
            Mutation::QualifyType(qualify_type::Data {
                entity_id: EntityId::try_from("d:dto.f:field.ty").unwrap(),
                new_ty: new_ty.clone(),
            })
            .execute(&mut api)
            .expect("mutation qualify type");
            assert_eq!(
                api.find_dto(&EntityId::new_unqualified("dto"))
                    .unwrap()
                    .field("field")
                    .unwrap()
                    .ty,
                new_ty
            )
        }

        #[test]
        fn error_not_api_type() {
            let result = run_test(
                r#"
            struct dto {
                field: SomeType
            }
            "#,
                "dto.field",
                TypeRef::new(Type::Bool, Semantics::Value),
            );
            assert!(result.is_err());
        }

        #[test]
        fn error_not_found() {
            let result = run_test(
                r#"
            struct dto {
                field: SomeType
            }
            "#,
                "i.dont.exist",
                TypeRef::new(Type::Bool, Semantics::Value),
            );
            assert!(result.is_err());
        }

        fn run_test(data: &str, id: &str, new_ty: TypeRef) -> Result<()> {
            let mut exe = TestExecutor::new(data);
            let mut api = exe.api();
            Mutation::QualifyType(qualify_type::Data {
                entity_id: EntityId::new_unqualified(id),
                new_ty,
            })
            .execute(&mut api)
        }
    }
}
