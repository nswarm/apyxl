mod mutation;

use std::fmt::Debug;

use itertools::Itertools;
use thiserror::Error;

pub use crate::model::validate::mutation::Mutation;
use crate::model::{entity, Api, EntityId, EntityType, Field, Type, TypeRef, UNDEFINED_NAMESPACE};

#[derive(Error, Debug, Eq, PartialEq)]
pub enum ValidationError {
    #[error(
        "Invalid namespace found at path {0}. Only the root namespace can be named {}.",
        UNDEFINED_NAMESPACE
    )]
    InvalidNamespaceName(EntityId),

    #[error("Invalid DTO name within namespace '{0}', index #{1}. DTO names cannot be empty.")]
    InvalidDtoName(EntityId, usize),

    #[error("Invalid RPC name within namespace '{0}', index #{1}. RPC names cannot be empty.")]
    InvalidRpcName(EntityId, usize),

    #[error("Invalid enum name within namespace '{0}', index #{1}. Enum names cannot be empty.")]
    InvalidEnumName(EntityId, usize),

    #[error("Invalid type alias name within namespace '{0}', index #{1}. Type alias names cannot be empty.")]
    InvalidTypeAliasName(EntityId, usize),

    #[error("Invalid field name at '{0}', index {1}. Field names cannot be empty.")]
    InvalidFieldName(EntityId, usize),

    #[error("Invalid enum value name at '{0}', index {1}. Enum value names cannot be empty.")]
    InvalidEnumValueName(EntityId, usize),

    #[error("Invalid field type at '{0}'. Type '{1}' must be a valid DTO, enum, or type alias in the API.")]
    InvalidFieldType(EntityId, EntityId),

    #[error(
        "Invalid field type '{0}::{1}', index {2}. Type '{3}' must be a valid DTO, enum, or type alias in the API."
    )]
    InvalidFieldOrParamType(EntityId, String, usize, EntityId),

    #[error("Invalid return type for RPC {0}. Type '{1}' must be a valid DTO, enum, or type alias in the API.")]
    InvalidRpcReturnType(EntityId, EntityId),

    #[error("Invalid target type for type alias {0}. Type '{1}' must be a valid DTO, enum, or type alias in the API.")]
    InvalidTypeAliasTargetType(EntityId, EntityId),

    #[error("Duplicate DTO, enum, or type alias definition: '{0}'")]
    DuplicateDtoOrEnumOrAlias(EntityId),

    #[error("Duplicate RPC or field definition: '{0}'")]
    DuplicateRpcOrField(EntityId),

    #[error("Duplicate enum value name within enum '{0}': '{1}'")]
    DuplicateEnumValue(EntityId, String),

    #[error("Duplicate field name within entity '{0}': '{1}'")]
    DuplicateFieldName(EntityId, String),
}

pub type ValidationResult = Result<Option<Mutation>, ValidationError>;

pub fn namespace_names(api: &Api, namespace_id: EntityId) -> Vec<ValidationResult> {
    api.find_namespace(&namespace_id)
        .expect("namespace must exist in api")
        .namespaces()
        .map(|child| {
            if child.name == UNDEFINED_NAMESPACE {
                Err(ValidationError::InvalidNamespaceName(
                    namespace_id
                        .child(EntityType::Namespace, &child.name)
                        .unwrap(),
                ))
            } else {
                Ok(None)
            }
        })
        .collect_vec()
}

pub fn no_duplicate_dto_enum_alias(api: &Api, namespace_id: EntityId) -> Vec<ValidationResult> {
    let namespace = api
        .find_namespace(&namespace_id)
        .expect("namespace must exist in api");
    let dto_names = namespace.dtos().map(|dto| dto.name);
    let enum_names = namespace.enums().map(|en| en.name);
    let alias_names = namespace.ty_aliases().map(|alias| alias.name);
    dto_names
        .chain(enum_names)
        .chain(alias_names)
        .duplicates()
        .map(|name| {
            Err(ValidationError::DuplicateDtoOrEnumOrAlias(
                namespace_id.to_unqualified().child_unqualified(name),
            ))
        })
        .collect_vec()
}

pub fn no_duplicate_rpc_or_field(api: &Api, namespace_id: EntityId) -> Vec<ValidationResult> {
    let namespace = api
        .find_namespace(&namespace_id)
        .expect("namespace must exist in api");
    let rpc_names = namespace.rpcs().map(|rpc| rpc.name.as_ref());
    let field_names = namespace.fields().map(|field| field.name);
    rpc_names
        .chain(field_names)
        .duplicates()
        .map(|name| {
            Err(ValidationError::DuplicateRpcOrField(
                namespace_id.to_unqualified().child_unqualified(name),
            ))
        })
        .collect_vec()
}

pub fn dto_names(api: &Api, namespace_id: EntityId) -> Vec<ValidationResult> {
    api.find_namespace(&namespace_id)
        .expect("namespace must exist in api")
        .dtos()
        .enumerate()
        .map(|(i, dto)| {
            if dto.name.is_empty() {
                Err(ValidationError::InvalidDtoName(namespace_id.clone(), i))
            } else {
                Ok(None)
            }
        })
        .collect_vec()
}

pub fn dto_field_names(api: &Api, namespace_id: EntityId) -> Vec<ValidationResult> {
    api.find_namespace(&namespace_id)
        .expect("namespace must exist in api")
        .dtos()
        .flat_map(|dto| {
            field_names(
                &dto.fields,
                namespace_id.child(EntityType::Dto, dto.name).unwrap(),
            )
        })
        .collect_vec()
}

pub fn dto_field_names_no_duplicates(api: &Api, namespace_id: EntityId) -> Vec<ValidationResult> {
    api.find_namespace(&namespace_id)
        .expect("namespace must exist in api")
        .dtos()
        .flat_map(|dto| {
            duplicate_field_names(
                &dto.fields,
                namespace_id.child(EntityType::Dto, dto.name).unwrap(),
            )
        })
        .collect_vec()
}

pub fn rpc_names(api: &Api, namespace_id: EntityId) -> Vec<ValidationResult> {
    api.find_namespace(&namespace_id)
        .expect("namespace must exist in api")
        .rpcs()
        .enumerate()
        .map(|(i, rpc)| {
            if rpc.name.is_empty() {
                Err(ValidationError::InvalidRpcName(namespace_id.clone(), i))
            } else {
                Ok(None)
            }
        })
        .collect_vec()
}

pub fn rpc_param_names(api: &Api, namespace_id: EntityId) -> Vec<ValidationResult> {
    api.find_namespace(&namespace_id)
        .expect("namespace must exist in api")
        .rpcs()
        .flat_map(|rpc| {
            field_names(
                &rpc.params,
                namespace_id.child(EntityType::Rpc, &rpc.name).unwrap(),
            )
        })
        .collect_vec()
}

pub fn rpc_param_names_no_duplicates(api: &Api, namespace_id: EntityId) -> Vec<ValidationResult> {
    api.find_namespace(&namespace_id)
        .expect("namespace must exist in api")
        .rpcs()
        .flat_map(|rpc| {
            duplicate_field_names(
                &rpc.params,
                namespace_id.child(EntityType::Rpc, &rpc.name).unwrap(),
            )
        })
        .collect_vec()
}

pub fn enum_names(api: &Api, namespace_id: EntityId) -> Vec<ValidationResult> {
    api.find_namespace(&namespace_id)
        .expect("namespace must exist in api")
        .enums()
        .enumerate()
        .map(|(i, en)| {
            if en.name.is_empty() {
                Err(ValidationError::InvalidEnumName(namespace_id.clone(), i))
            } else {
                Ok(None)
            }
        })
        .collect_vec()
}

pub fn enum_value_names(api: &Api, namespace_id: EntityId) -> Vec<ValidationResult> {
    api.find_namespace(&namespace_id)
        .expect("namespace must exist in api")
        .enums()
        .flat_map(|en| {
            en.values.iter().enumerate().map(|(i, value)| {
                if value.name.is_empty() {
                    Err(ValidationError::InvalidEnumValueName(
                        namespace_id.child(EntityType::Enum, en.name).unwrap(),
                        i,
                    ))
                } else {
                    Ok(None)
                }
            })
        })
        .collect_vec()
}

pub fn no_duplicate_enum_value_names(api: &Api, namespace_id: EntityId) -> Vec<ValidationResult> {
    api.find_namespace(&namespace_id)
        .expect("namespace must exist in api")
        .enums()
        .flat_map(|en| {
            en.values
                .iter()
                .duplicates_by(|value| value.name)
                .map(|value| {
                    Err(ValidationError::DuplicateEnumValue(
                        namespace_id.child(EntityType::Enum, en.name).unwrap(),
                        value.name.to_string(),
                    ))
                })
        })
        .collect_vec()
}

pub fn ty_alias_names(api: &Api, namespace_id: EntityId) -> Vec<ValidationResult> {
    api.find_namespace(&namespace_id)
        .expect("namespace must exist in api")
        .ty_aliases()
        .enumerate()
        .map(|(i, alias)| {
            if alias.name.is_empty() {
                Err(ValidationError::InvalidTypeAliasName(
                    namespace_id.clone(),
                    i,
                ))
            } else {
                Ok(None)
            }
        })
        .collect_vec()
}

pub fn field_names(fields: &[Field], parent_entity_id: EntityId) -> Vec<ValidationResult> {
    fields
        .iter()
        .enumerate()
        .map(|(i, field)| {
            if field.name.is_empty() {
                Err(ValidationError::InvalidFieldName(
                    parent_entity_id.clone(),
                    i,
                ))
            } else {
                Ok(None)
            }
        })
        .collect_vec()
}

pub fn duplicate_field_names(
    fields: &[Field],
    parent_entity_id: EntityId,
) -> Vec<ValidationResult> {
    fields
        .iter()
        .duplicates_by(|field| field.name)
        .map(|field| {
            Err(ValidationError::DuplicateFieldName(
                parent_entity_id.clone(),
                field.name.to_string(),
            ))
        })
        .collect_vec()
}

pub fn dto_field_types(api: &Api, namespace_id: EntityId) -> Vec<ValidationResult> {
    api.find_namespace(&namespace_id)
        .expect("namespace must exist in api")
        .dtos()
        .flat_map(|dto| {
            let dto_id = namespace_id.child(EntityType::Dto, dto.name).unwrap();
            field_list_types(api, &dto.fields, namespace_id.clone(), dto_id)
        })
        .collect_vec()
}

pub fn rpc_param_types(api: &Api, namespace_id: EntityId) -> Vec<ValidationResult> {
    api.find_namespace(&namespace_id)
        .expect("namespace must exist in api")
        .rpcs()
        .flat_map(|rpc| {
            let rpc_id = namespace_id.child(EntityType::Rpc, &rpc.name).unwrap();
            field_list_types(api, &rpc.params, namespace_id.clone(), rpc_id)
        })
        .collect_vec()
}

pub fn rpc_return_types(api: &Api, namespace_id: EntityId) -> Vec<ValidationResult> {
    api.find_namespace(&namespace_id)
        .expect("namespace must exist in api")
        .rpcs()
        .filter_map(|rpc| rpc.return_type.as_ref().map(|ty| (&rpc.name, ty)))
        .map(|(rpc_name, return_type)| {
            let rpc_id = namespace_id.child(EntityType::Rpc, rpc_name).unwrap();
            let return_ty_id = rpc_id
                .child(EntityType::Type, entity::subtype::RETURN_TY)
                .unwrap();
            match qualify_type(api, &namespace_id, return_type) {
                Ok(Some(qualified_ty)) => {
                    Ok(Some(Mutation::new_qualify_type(return_ty_id, qualified_ty)))
                }
                Err(err_entity_id) => {
                    Err(ValidationError::InvalidRpcReturnType(rpc_id, err_entity_id))
                }
                _ => Ok(None),
            }
        })
        .collect_vec()
}

/// Field contained as a list inside other Entities like Dto fields or Rpc params.
pub fn field_list_types<'a, 'b: 'a>(
    api: &'b Api<'a>,
    fields: &[Field],
    namespace_id: EntityId,
    parent_entity_id: EntityId,
) -> Vec<ValidationResult> {
    fields
        .iter()
        .enumerate()
        .map(|(i, field)| {
            let field_id = parent_entity_id
                .child(EntityType::Field, field.name)
                .unwrap();
            let ty_id = field_id
                .child(EntityType::Type, entity::subtype::TY)
                .unwrap();
            match qualify_type(api, &namespace_id, &field.ty) {
                Ok(Some(qualified_ty)) => Ok(Some(Mutation::new_qualify_type(ty_id, qualified_ty))),
                Err(err_entity_id) => Err(ValidationError::InvalidFieldOrParamType(
                    parent_entity_id.clone(),
                    field.name.to_string(),
                    i,
                    err_entity_id,
                )),
                _ => Ok(None),
            }
        })
        .collect_vec()
}

/// Top level fields inside a namespace.
pub fn field_types<'a, 'b: 'a>(api: &'b Api<'a>, namespace_id: EntityId) -> Vec<ValidationResult> {
    api.find_namespace(&namespace_id)
        .expect("namespace must exist in api")
        .fields()
        .map(|field| {
            let field_id = namespace_id.child(EntityType::Field, field.name).unwrap();
            let ty_id = field_id
                .child(EntityType::Type, entity::subtype::TY)
                .unwrap();
            match qualify_type(api, &namespace_id, &field.ty) {
                Ok(Some(qualified_ty)) => Ok(Some(Mutation::new_qualify_type(ty_id, qualified_ty))),
                Err(err_entity_id) => {
                    Err(ValidationError::InvalidFieldType(field_id, err_entity_id))
                }
                _ => Ok(None),
            }
        })
        .collect_vec()
}

pub fn ty_alias_target_type(api: &Api, namespace_id: EntityId) -> Vec<ValidationResult> {
    api.find_namespace(&namespace_id)
        .expect("namespace must exist in api")
        .ty_aliases()
        .map(|alias| {
            let alias_id = namespace_id
                .child(EntityType::TypeAlias, alias.name)
                .unwrap();
            let target_ty_id = alias_id
                .child(EntityType::Type, entity::subtype::TY_ALIAS_TARGET)
                .unwrap();
            match qualify_type(api, &namespace_id, &alias.target_ty) {
                Ok(Some(qualified_ty)) => {
                    Ok(Some(Mutation::new_qualify_type(target_ty_id, qualified_ty)))
                }
                Err(err_entity_id) => Err(ValidationError::InvalidTypeAliasTargetType(
                    alias_id,
                    err_entity_id,
                )),
                _ => Ok(None),
            }
        })
        .collect_vec()
}

/// Returns a [TypeRef] with all [EntityId]s qualified, recursively. If an [EntityId] does not exist
/// in the `api`, it returns the [EntityId] which could not be qualified as an error.
/// If there are no [EntityId]s in the [TypeRef] (i.e. it's all primitives), returns Ok(None).
fn qualify_type(
    api: &Api,
    namespace_id: &EntityId,
    ty: &TypeRef,
) -> Result<Option<TypeRef>, EntityId> {
    // This fn is recursive to support nested types like `Vec<EnumA, Map<EnumB, Vec<DtoA>>>`
    // It digs into the [Type] `ty` until it runs into a [Type::Api] that has an [EntityId] to
    // be qualified and returns the qualified version. On the way back up the stack each [Type]
    // will wrap the result in its own enum variant so that by the time we reach the top, it has
    // the same structure as the input type `ty`.
    match &ty.value {
        Type::Api(id) => {
            let qualified_id = api
                .find_qualified_type_relative(namespace_id, id)
                .ok_or(id.clone())?;
            return Ok(Some(TypeRef::new(Type::Api(qualified_id), ty.semantics)));
        }

        Type::Array(ty) => {
            return qualify_type(api, namespace_id, ty)
                .map(|opt| opt.map(|arr_ty| TypeRef::new_array(arr_ty, ty.semantics)))
        }

        Type::Optional(ty) => {
            return qualify_type(api, namespace_id, ty)
                .map(|opt| opt.map(|opt_ty| TypeRef::new_optional(opt_ty, ty.semantics)))
        }

        Type::Map { key, value } => {
            let key_ty = qualify_type(api, namespace_id, key)?;
            let value_ty = qualify_type(api, namespace_id, value)?;
            return if key_ty.is_some() || value_ty.is_some() {
                Ok(Some(TypeRef::new_map(
                    key_ty.unwrap_or((**key).clone()),
                    value_ty.unwrap_or((**value).clone()),
                    ty.semantics,
                )))
            } else {
                Ok(None)
            };
        }

        // Nothing to qualify.
        Type::Bool => {}
        Type::U8 => {}
        Type::U16 => {}
        Type::U32 => {}
        Type::U64 => {}
        Type::U128 => {}
        Type::USIZE => {}
        Type::I8 => {}
        Type::I16 => {}
        Type::I32 => {}
        Type::I64 => {}
        Type::I128 => {}
        Type::F8 => {}
        Type::F16 => {}
        Type::F32 => {}
        Type::F64 => {}
        Type::F128 => {}
        Type::StringView => {}
        Type::String => {}
        Type::Bytes => {}
        Type::User(_) => {}
    }
    Ok(None)
}

/// Calls the function `action` for each [Namespace] in the `api`. `action` will be passed the [Namespace]
/// currently being operated on and a [EntityId] to that namespace within the overall hierarchy.
///
/// `'a` is the lifetime of the [Api] bound.
/// `'b` is the lifetime of the [Builder::build] process.
pub(crate) fn recurse_api<'a, 'b, Action>(api: &'b Api<'a>, action: Action) -> Vec<ValidationResult>
where
    'b: 'a,
    Action: Clone + Fn(&'b Api<'a>, EntityId) -> Vec<ValidationResult>,
{
    recurse_namespaces(api, EntityId::default(), action)
}

fn recurse_namespaces<'a, 'b, Action>(
    api: &'b Api<'a>,
    namespace_id: EntityId,
    action: Action,
) -> Vec<ValidationResult>
where
    'b: 'a,
    Action: Clone + Fn(&'b Api<'a>, EntityId) -> Vec<ValidationResult>,
{
    let namespace = api
        .find_namespace(&namespace_id)
        .expect("namespace must exist in api");

    let dto_results = namespace.dtos().flat_map(|dto| match &dto.namespace {
        Some(_) => recurse_namespaces(
            api,
            namespace_id.child(EntityType::Dto, dto.name).unwrap(),
            action.clone(),
        ),
        _ => Vec::new(),
    });

    let child_results = namespace.namespaces().flat_map(|child| {
        let child_ty = if child.is_virtual {
            EntityType::Dto
        } else {
            EntityType::Namespace
        };
        recurse_namespaces(
            api,
            namespace_id.child(child_ty, &child.name).unwrap(),
            action.clone(),
        )
    });

    dto_results
        .chain(child_results)
        .chain(action(api, namespace_id.clone()))
        .collect_vec()
}

#[cfg(test)]
mod tests {
    // note: many validators tested via actual code paths in builder.

    use crate::model::validate::rpc_return_types;
    use crate::model::EntityId;
    use crate::test_util::executor::TestExecutor;

    #[test]
    fn test_rpc_return_types() {
        let mut exe = TestExecutor::new(
            r#"
            mod ns0 {
                mod ns1 {
                    mod ns2 {
                        fn rpc() -> other0::other1::dto1 {}
                        fn rpc() -> other0::dto2 {}
                        fn rpc() -> dto3 {}
                        fn rpc() -> ns3::dto4 {}
                        fn rpc() -> en {}

                        struct dto3 {}
                        enum en {}

                        mod ns3 {
                            struct dto4 {}
                        }
                    }
                }
                mod other0 {
                    mod other1 {
                        struct dto1 {}
                    }
                    struct dto2 {}
                }
            }
            "#,
        );
        let api = exe.api();

        let namespace_id = EntityId::try_from("ns0.ns1.ns2").unwrap();
        assert!(rpc_return_types(&api, namespace_id)
            .iter()
            .all(|result| result.is_ok()));
    }

    mod qualify_type {
        use crate::model::validate::qualify_type;
        use crate::model::{EntityId, Semantics, Type, TypeRef};
        use crate::test_util::executor::TestExecutor;

        #[test]
        fn primitive() {
            run_test(
                "",
                &EntityId::default(),
                &TypeRef::new(Type::String, Semantics::Value),
                None,
            );
        }

        #[test]
        fn api() {
            run_test(
                "mod ns { struct dto {} }",
                &EntityId::default(),
                &TypeRef::new(
                    Type::Api(EntityId::new_unqualified("ns.dto")),
                    Semantics::Value,
                ),
                Some(TypeRef::new_api("ns.d:dto", Semantics::Value).unwrap()),
            );
        }

        #[test]
        fn array_primitive() {
            run_test(
                "",
                &EntityId::default(),
                &TypeRef::new_array(
                    TypeRef::new(Type::String, Semantics::Value),
                    Semantics::Value,
                ),
                None,
            );
        }

        #[test]
        fn array_complex() {
            run_test(
                "mod ns { struct dto {} }",
                &EntityId::default(),
                &TypeRef::new_array(
                    TypeRef::new(
                        Type::Api(EntityId::new_unqualified("ns.dto")),
                        Semantics::Value,
                    ),
                    Semantics::Value,
                ),
                Some(TypeRef::new_array(
                    TypeRef::new_api("ns.d:dto", Semantics::Value).unwrap(),
                    Semantics::Value,
                )),
            );
        }

        #[test]
        fn optional_primitive() {
            run_test(
                "",
                &EntityId::default(),
                &TypeRef::new_optional(
                    TypeRef::new(Type::String, Semantics::Value),
                    Semantics::Value,
                ),
                None,
            );
        }

        #[test]
        fn optional_complex() {
            run_test(
                "mod ns { struct dto {} }",
                &EntityId::default(),
                &TypeRef::new_optional(
                    TypeRef::new(
                        Type::Api(EntityId::new_unqualified("ns.dto")),
                        Semantics::Value,
                    ),
                    Semantics::Value,
                ),
                Some(TypeRef::new_optional(
                    TypeRef::new_api("ns.d:dto", Semantics::Value).unwrap(),
                    Semantics::Value,
                )),
            );
        }

        #[test]
        fn map_primitive() {
            run_test(
                "",
                &EntityId::default(),
                &TypeRef::new_map(
                    TypeRef::new(Type::String, Semantics::Value),
                    TypeRef::new(Type::String, Semantics::Value),
                    Semantics::Value,
                ),
                None,
            );
        }

        #[test]
        fn map_complex() {
            run_test(
                r#"
                mod ns0 {
                    struct dto {}
                    mod ns1 {
                        enum en {}
                    }
                }
                "#,
                &EntityId::default(),
                &TypeRef::new_map(
                    TypeRef::new(
                        Type::Api(EntityId::new_unqualified("ns0.ns1.en")),
                        Semantics::Value,
                    ),
                    TypeRef::new(
                        Type::Api(EntityId::new_unqualified("ns0.dto")),
                        Semantics::Value,
                    ),
                    Semantics::Value,
                ),
                Some(TypeRef::new_map(
                    TypeRef::new_api("ns0.ns1.e:en", Semantics::Value).unwrap(),
                    TypeRef::new_api("ns0.d:dto", Semantics::Value).unwrap(),
                    Semantics::Value,
                )),
            );
        }

        #[test]
        fn nested() {
            run_test(
                r#"
                mod ns0 {
                    struct dto {}
                    mod ns1 {
                        enum en {}
                    }
                }
                "#,
                &EntityId::default(),
                &TypeRef::new_array(
                    TypeRef::new_map(
                        TypeRef::new(
                            Type::Api(EntityId::new_unqualified("ns0.ns1.en")),
                            Semantics::Value,
                        ),
                        TypeRef::new(
                            Type::Api(EntityId::new_unqualified("ns0.dto")),
                            Semantics::Value,
                        ),
                        Semantics::Value,
                    ),
                    Semantics::Value,
                ),
                Some(TypeRef::new_array(
                    TypeRef::new_map(
                        TypeRef::new_api("ns0.ns1.e:en", Semantics::Value).unwrap(),
                        TypeRef::new_api("ns0.d:dto", Semantics::Value).unwrap(),
                        Semantics::Value,
                    ),
                    Semantics::Value,
                )),
            );
        }

        #[test]
        fn error() {
            run_test_err(
                "",
                &EntityId::default(),
                &TypeRef::new(
                    Type::Api(EntityId::new_unqualified("dto")),
                    Semantics::Value,
                ),
            );
        }

        #[test]
        fn error_nested() {
            run_test_err(
                "",
                &EntityId::default(),
                &TypeRef::new_array(
                    TypeRef::new_map(
                        TypeRef::new(Type::String, Semantics::Value),
                        TypeRef::new(
                            Type::Api(EntityId::new_unqualified("dto")),
                            Semantics::Value,
                        ),
                        Semantics::Value,
                    ),
                    Semantics::Value,
                ),
            );
        }

        fn run_test(
            data: &str,
            namespace_id: &EntityId,
            unqualified: &TypeRef,
            expected: Option<TypeRef>,
        ) {
            let mut exe = TestExecutor::new(data);
            let api = exe.api();
            let qualified = qualify_type(&api, namespace_id, unqualified).unwrap();
            assert_eq!(qualified, expected);
        }

        fn run_test_err(data: &str, namespace_id: &EntityId, unqualified: &TypeRef) {
            let mut exe = TestExecutor::new(data);
            let api = exe.api();
            assert!(qualify_type(&api, namespace_id, unqualified).is_err());
        }
    }

    mod field_list_types {
        use crate::model::api::validate::field_list_types;
        use crate::model::EntityId;
        use crate::test_util::executor::TestExecutor;

        #[test]
        fn absolute_path_from_top() {
            run_test(
                r#"
                struct dto0 {
                    field: ns0::ns1::dto1,
                    field: ns0::dto2,
                }
                mod ns0 {
                    mod ns1 {
                        struct dto1 {}
                    }
                    struct dto2 {}
                }
                "#,
                &EntityId::try_from("d:dto0").unwrap(),
            );
        }

        #[test]
        fn absolute_path_within_ns() {
            run_test(
                r#"
                mod ns0 {
                    mod ns1 {
                        struct dto0 {
                            field: ns0::ns1::dto1,
                            field: ns1::dto1,
                        }
                        struct dto1 {}
                    }
                }
                "#,
                &EntityId::try_from("ns0.ns1.d:dto0").unwrap(),
            );
        }

        #[test]
        fn enum_type() {
            run_test(
                r#"
                mod ns0 {
                    mod ns1 {
                        struct dto0 {
                            field0: en,
                        }
                        enum en {}
                    }
                }
                "#,
                &EntityId::try_from("ns0.ns1.d:dto0").unwrap(),
            );
        }

        #[test]
        fn relative_path_local() {
            run_test(
                r#"
                mod ns0 {
                    mod ns1 {
                        struct dto0 {
                            field0: dto1,
                        }
                        struct dto1 {}
                    }
                }
                "#,
                &EntityId::try_from("ns0.ns1.d:dto0").unwrap(),
            );
        }

        #[test]
        fn relative_path_up() {
            run_test(
                r#"
                mod ns0 {
                    mod ns1 {
                        struct dto0 {
                            field: dto1,
                            field: dto2,
                        }
                    }
                    struct dto1 {}
                }
                struct dto2 {}
                "#,
                &EntityId::try_from("ns0.ns1.d:dto0").unwrap(),
            );
        }

        #[test]
        fn relative_path_down() {
            run_test(
                r#"
                mod ns0 {
                    struct dto0 {
                        field: ns1::dto1,
                        field: ns1::ns2::dto2,
                    }
                    mod ns1 {
                        struct dto1 {}
                        mod ns2 {
                            struct dto2 {}
                        }
                    }
                }
                "#,
                &EntityId::try_from("ns0.d:dto0").unwrap(),
            );
        }

        #[test]
        fn relative_path_sibling() {
            run_test(
                r#"
                mod ns0 {
                    mod ns1 {
                        struct dto0 {
                            field: ns2::dto1,
                            field: ns3::ns4::dto2,
                        }
                    }
                    mod ns2 {
                        struct dto1 {}
                    }
                    mod ns3 {
                        mod ns4 {
                            struct dto2 {}
                        }
                    }
                }
                "#,
                &EntityId::try_from("ns0.ns1.d:dto0").unwrap(),
            );
        }

        #[test]
        fn relative_path_cousin() {
            run_test(
                r#"
                mod ns0 {
                    mod ns1 {
                        mod ns2 {
                            struct dto0 {
                                field: other0::other1::dto1,
                                field: other0::dto2,
                            }
                        }
                    }
                    mod other0 {
                        mod other1 {
                            struct dto1 {}
                        }
                        struct dto2 {}
                    }
                }
                "#,
                &EntityId::try_from("ns0.ns1.ns2.d:dto0").unwrap(),
            );
        }

        fn run_test(input_data: &str, source_dto: &EntityId) {
            let mut exe = TestExecutor::new(input_data);
            let api = exe.api();

            assert!(field_list_types(
                &api,
                &api.find_dto(source_dto)
                    .expect("couldn't find source dto")
                    .fields,
                source_dto.parent().expect("dto has no parent"),
                source_dto.clone(),
            )
            .iter()
            .all(|result| result.is_ok()));
        }
    }

    mod recurse_api {
        use crate::model::validate::recurse_api;
        use crate::model::{
            Api, Dto, EntityId, Enum, Field, Namespace, NamespaceChild, Rpc, Semantics, Type,
            TypeRef, UNDEFINED_NAMESPACE,
        };
        use std::borrow::Cow;
        use std::cell::RefCell;
        use std::rc::Rc;

        // todo more tests here

        // Some of these don't work with the rust parser so we need to manually create the Api...

        #[test]
        fn recurse_into_existing_dto_namespace() {
            let api = Api {
                name: Cow::Borrowed(UNDEFINED_NAMESPACE),
                children: vec![NamespaceChild::Dto(Dto {
                    name: "dto",
                    namespace: Some(Namespace {
                        children: vec![
                            NamespaceChild::Field(Field {
                                name: "field",
                                ty: TypeRef::new(Type::U32, Semantics::Value),
                                attributes: Default::default(),
                                is_static: true,
                            }),
                            NamespaceChild::Rpc(Rpc {
                                name: Cow::Borrowed("rpc"),
                                is_static: true,
                                ..Default::default()
                            }),
                            NamespaceChild::Enum(Enum {
                                name: "en",
                                ..Default::default()
                            }),
                        ],
                        ..Default::default()
                    }),
                    ..Default::default()
                })],
                is_virtual: false,
                ..Default::default()
            };

            let ids = collect_recurse_ids(&api);
            assert_eq!(
                &ids,
                &[EntityId::default(), EntityId::try_from("d:dto").unwrap(),]
            );
        }

        fn collect_recurse_ids(api: &Api) -> Vec<EntityId> {
            let ids = Rc::new(RefCell::new(Vec::new()));

            recurse_api(api, |_, id| {
                ids.borrow_mut().push(id);
                Vec::new()
            });

            Rc::into_inner(ids).unwrap().into_inner()
        }
    }
}
