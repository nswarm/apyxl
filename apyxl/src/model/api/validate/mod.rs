use std::fmt::Debug;

use itertools::Itertools;
use thiserror::Error;

use crate::model::{Api, EntityId, Field, Type, UNDEFINED_NAMESPACE};

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

    #[error("Invalid field name at '{0}', index {1}. Field names cannot be empty.")]
    InvalidFieldName(EntityId, usize),

    #[error("Invalid enum value name at '{0}', index {1}. Enum value names cannot be empty.")]
    InvalidEnumValueName(EntityId, usize),

    #[error(
        "Invalid field type '{0}::{1}', index {2}. Type '{3}' must be a valid DTO or enum in the API."
    )]
    InvalidFieldType(EntityId, String, usize, EntityId),

    #[error("Invalid return type for RPC {0}. Type '{1}' must be a valid DTO or enum in the API.")]
    InvalidRpcReturnType(EntityId, EntityId),

    #[error("Duplicate DTO or enum definition: '{0}'")]
    DuplicateDtoOrEnum(EntityId),

    #[error("Duplicate RPC definition: '{0}'")]
    DuplicateRpc(EntityId),

    #[error("Duplicate enum value name within enum '{1}': '{0}'")]
    DuplicateEnumValue(EntityId, String),
}

pub fn namespace_names(api: &Api, namespace_id: EntityId) -> Vec<ValidationError> {
    api.find_namespace(&namespace_id)
        .expect("namespace must exist in api")
        .namespaces()
        .filter_map(|child| {
            if child.name == UNDEFINED_NAMESPACE {
                Some(ValidationError::InvalidNamespaceName(
                    namespace_id.child_unqualified(&child.name).to_owned(),
                ))
            } else {
                None
            }
        })
        .collect_vec()
}

pub fn no_duplicate_dto_enums(api: &Api, namespace_id: EntityId) -> Vec<ValidationError> {
    let namespace = api
        .find_namespace(&namespace_id)
        .expect("namespace must exist in api");
    let dto_names = namespace.dtos().map(|dto| dto.name);
    let enum_names = namespace.enums().map(|en| en.name);
    dto_names
        .chain(enum_names)
        .duplicates()
        .map(|name| {
            ValidationError::DuplicateDtoOrEnum(namespace_id.child_unqualified(name).to_owned())
        })
        .collect_vec()
}

pub fn no_duplicate_rpcs(api: &Api, namespace_id: EntityId) -> Vec<ValidationError> {
    api.find_namespace(&namespace_id)
        .expect("namespace must exist in api")
        .rpcs()
        .duplicates_by(|rpc| rpc.name)
        .map(|rpc| {
            ValidationError::DuplicateRpc(namespace_id.child_unqualified(rpc.name).to_owned())
        })
        .collect_vec()
}

pub fn dto_names(api: &Api, namespace_id: EntityId) -> Vec<ValidationError> {
    api.find_namespace(&namespace_id)
        .expect("namespace must exist in api")
        .dtos()
        .enumerate()
        .filter_map(|(i, dto)| {
            if dto.name.is_empty() {
                Some(ValidationError::InvalidDtoName(
                    namespace_id.clone().to_owned(),
                    i,
                ))
            } else {
                None
            }
        })
        .collect_vec()
}

pub fn dto_field_names(api: &Api, namespace_id: EntityId) -> Vec<ValidationError> {
    api.find_namespace(&namespace_id)
        .expect("namespace must exist in api")
        .dtos()
        .flat_map(|dto| field_names(&dto.fields, namespace_id.child_unqualified(dto.name)))
        .collect_vec()
}

pub fn rpc_names(api: &Api, namespace_id: EntityId) -> Vec<ValidationError> {
    api.find_namespace(&namespace_id)
        .expect("namespace must exist in api")
        .rpcs()
        .enumerate()
        .filter_map(|(i, rpc)| {
            if rpc.name.is_empty() {
                Some(ValidationError::InvalidRpcName(namespace_id.to_owned(), i))
            } else {
                None
            }
        })
        .collect_vec()
}

pub fn rpc_param_names(api: &Api, namespace_id: EntityId) -> Vec<ValidationError> {
    api.find_namespace(&namespace_id)
        .expect("namespace must exist in api")
        .rpcs()
        .flat_map(|rpc| field_names(&rpc.params, namespace_id.child_unqualified(rpc.name)))
        .collect_vec()
}

pub fn enum_names(api: &Api, namespace_id: EntityId) -> Vec<ValidationError> {
    api.find_namespace(&namespace_id)
        .expect("namespace must exist in api")
        .enums()
        .enumerate()
        .filter_map(|(i, en)| {
            if en.name.is_empty() {
                Some(ValidationError::InvalidEnumName(namespace_id.to_owned(), i))
            } else {
                None
            }
        })
        .collect_vec()
}

pub fn enum_value_names(api: &Api, namespace_id: EntityId) -> Vec<ValidationError> {
    api.find_namespace(&namespace_id)
        .expect("namespace must exist in api")
        .enums()
        .flat_map(|en| {
            en.values.iter().enumerate().filter_map(|(i, value)| {
                if value.name.is_empty() {
                    Some(ValidationError::InvalidEnumValueName(
                        namespace_id.child_unqualified(en.name),
                        i,
                    ))
                } else {
                    None
                }
            })
        })
        .collect_vec()
}

pub fn no_duplicate_enum_value_names(api: &Api, namespace_id: EntityId) -> Vec<ValidationError> {
    api.find_namespace(&namespace_id)
        .expect("namespace must exist in api")
        .enums()
        .flat_map(|en| {
            en.values
                .iter()
                .duplicates_by(|value| value.name)
                .map(|value| {
                    ValidationError::DuplicateEnumValue(
                        namespace_id.child_unqualified(en.name),
                        value.name.to_owned(),
                    )
                })
        })
        .collect_vec()
}

pub fn field_names(fields: &[Field], parent_entity_id: EntityId) -> Vec<ValidationError> {
    fields
        .iter()
        .enumerate()
        .filter_map(|(i, field)| {
            if field.name.is_empty() {
                Some(ValidationError::InvalidFieldName(
                    parent_entity_id.to_owned(),
                    i,
                ))
            } else {
                None
            }
        })
        .collect_vec()
}

pub fn dto_field_types(api: &Api, namespace_id: EntityId) -> Vec<ValidationError> {
    api.find_namespace(&namespace_id)
        .expect("namespace must exist in api")
        .dtos()
        .flat_map(|dto| {
            let dto_id = namespace_id.child_unqualified(dto.name);
            field_types(api, &dto.fields, namespace_id.clone(), dto_id)
        })
        .collect_vec()
}

pub fn rpc_param_types(api: &Api, namespace_id: EntityId) -> Vec<ValidationError> {
    api.find_namespace(&namespace_id)
        .expect("namespace must exist in api")
        .rpcs()
        .flat_map(|rpc| {
            let rpc_id = namespace_id.child_unqualified(rpc.name);
            field_types(api, &rpc.params, namespace_id.clone(), rpc_id)
        })
        .collect_vec()
}

pub fn rpc_return_types(api: &Api, namespace_id: EntityId) -> Vec<ValidationError> {
    api.find_namespace(&namespace_id)
        .expect("namespace must exist in api")
        .rpcs()
        .filter_map(|rpc| rpc.return_type.as_ref().map(|ty| (rpc.name, ty)))
        .filter_map(|(rpc_name, return_type)| {
            match fully_qualify_type(api, &namespace_id, return_type) {
                Ok(_) => None,
                Err(err_entity_id) => Some(ValidationError::InvalidRpcReturnType(
                    namespace_id.child_unqualified(rpc_name).to_owned(),
                    err_entity_id.to_owned(),
                )),
            }
        })
        .collect_vec()
}

pub fn field_types<'a, 'b: 'a>(
    api: &'b Api<'a>,
    fields: &[Field],
    namespace_id: EntityId,
    parent_entity_id: EntityId,
) -> Vec<ValidationError> {
    fields
        .iter()
        .enumerate()
        .filter_map(
            |(i, field)| match fully_qualify_type(api, &namespace_id, &field.ty) {
                Ok(_) => None,
                Err(err_entity_id) => Some(ValidationError::InvalidFieldType(
                    parent_entity_id.clone(),
                    field.name.to_string(),
                    i,
                    err_entity_id.to_owned(),
                )),
            },
        )
        .collect_vec()
}

fn fully_qualify_type(api: &Api, namespace_id: &EntityId, ty: &Type) -> Result<(), EntityId> {
    match ty {
        Type::Api(id) => {
            let fqt = api
                .find_type_relative(namespace_id.clone(), id)
                .ok_or(id.clone())?;
            // todo return mutation
            // *id = fqt;
        }

        Type::Array(ty) => fully_qualify_type(api, namespace_id, ty)?,
        Type::Optional(ty) => fully_qualify_type(api, namespace_id, ty)?,
        Type::Map { key, value } => {
            fully_qualify_type(api, namespace_id, key)?;
            fully_qualify_type(api, namespace_id, value)?;
        }

        // Nothing to qualify.
        Type::Bool => {}
        Type::U8 => {}
        Type::U16 => {}
        Type::U32 => {}
        Type::U64 => {}
        Type::U128 => {}
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
        Type::String => {}
        Type::Bytes => {}
        Type::User(_) => {}
    }
    Ok(())
}

/// Calls the function `action` for each [Namespace] in the `api`. `action` will be passed the [Namespace]
/// currently being operated on and a [EntityId] to that namespace within the overall hierarchy.
///
/// `'a` is the lifetime of the [Api] bound.
/// `'b` is the lifetime of the [Builder::build] process.
pub(crate) fn recurse_api<'a, 'b, Action>(api: &'b Api<'a>, action: Action) -> Vec<ValidationError>
where
    'b: 'a,
    Action: Copy + Fn(&'b Api<'a>, EntityId) -> Vec<ValidationError>,
{
    recurse_namespaces(api, EntityId::default(), action)
}

fn recurse_namespaces<'a, 'b, Action>(
    api: &'b Api<'a>,
    namespace_id: EntityId,
    action: Action,
) -> Vec<ValidationError>
where
    'b: 'a,
    Action: Copy + Fn(&'b Api<'a>, EntityId) -> Vec<ValidationError>,
{
    let namespace = api
        .find_namespace(&namespace_id)
        .expect("namespace must exist in api");

    let child_results = namespace.namespaces().flat_map(|child| {
        recurse_namespaces(api, namespace_id.child_unqualified(&child.name), action)
    });

    child_results
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
    fn happy_path() {
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

        let namespace_id = EntityId::new_unqualified("ns0.ns1.ns2");
        assert_eq!(rpc_return_types(&api, namespace_id), vec![]);
    }

    mod field_types {

        use crate::model::api::validate::field_types;
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
                &EntityId::new_unqualified("dto0"),
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
                &EntityId::new_unqualified("ns0.ns1.dto0"),
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
                &EntityId::new_unqualified("ns0.ns1.dto0"),
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
                &EntityId::new_unqualified("ns0.ns1.dto0"),
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
                &EntityId::new_unqualified("ns0.ns1.dto0"),
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
                &EntityId::new_unqualified("ns0.dto0"),
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
                &EntityId::new_unqualified("ns0.ns1.dto0"),
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
                &EntityId::new_unqualified("ns0.ns1.ns2.dto0"),
            );
        }

        fn run_test(input_data: &str, source_dto: &EntityId) {
            let mut exe = TestExecutor::new(input_data);
            let api = exe.api();

            assert_eq!(
                field_types(
                    &api,
                    &api.find_dto(source_dto)
                        .expect("couldn't find source dto")
                        .fields,
                    source_dto.parent().expect("dto has no parent"),
                    source_dto.clone(),
                ),
                vec![]
            );
        }
    }
}
