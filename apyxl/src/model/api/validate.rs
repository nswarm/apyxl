use std::fmt::Debug;

use itertools::Itertools;
use thiserror::Error;

use crate::model::{Api, EntityId, Field, Namespace, UNDEFINED_NAMESPACE};

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

    #[error("Invalid field name at '{0}', index {1}. Field names cannot be empty.")]
    InvalidFieldName(EntityId, usize),

    #[error(
        "Invalid field type '{0}::{1}', index {2}. Type '{3}' must be a valid DTO in the API."
    )]
    InvalidFieldType(EntityId, String, usize, EntityId),

    #[error("Invalid return type for RPC {0}. Type '{1}' must be a valid DTO in the API.")]
    InvalidRpcReturnType(EntityId, EntityId),

    #[error("Duplicate DTO definition: '{0}'")]
    DuplicateDto(EntityId),

    #[error("Duplicate RPC definition: '{0}'")]
    DuplicateRpc(EntityId),
}

pub fn namespace_names<'a, 'b: 'a>(
    _: &'b Api<'a>,
    namespace: &'b Namespace<'a>,
    namespace_id: EntityId,
) -> impl Iterator<Item = ValidationError> + 'b {
    namespace.namespaces().filter_map(move |child| {
        if child.name == UNDEFINED_NAMESPACE {
            Some(ValidationError::InvalidNamespaceName(
                namespace_id.child(&child.name).to_owned(),
            ))
        } else {
            None
        }
    })
}

pub fn no_duplicate_dtos<'a, 'b>(
    _: &'b Api<'a>,
    namespace: &'b Namespace<'a>,
    namespace_id: EntityId,
) -> impl Iterator<Item = ValidationError> + 'b {
    namespace
        .dtos()
        .duplicates_by(|dto| dto.name)
        .map(move |dto| ValidationError::DuplicateDto(namespace_id.child(dto.name).to_owned()))
}

pub fn no_duplicate_rpcs<'a, 'b>(
    _: &'b Api<'a>,
    namespace: &'b Namespace<'a>,
    namespace_id: EntityId,
) -> impl Iterator<Item = ValidationError> + 'b {
    namespace
        .rpcs()
        .duplicates_by(|rpc| rpc.name)
        .map(move |rpc| ValidationError::DuplicateRpc(namespace_id.child(rpc.name).to_owned()))
}

pub fn dto_names<'a, 'b>(
    _: &'b Api<'a>,
    namespace: &'b Namespace<'a>,
    namespace_id: EntityId,
) -> impl Iterator<Item = ValidationError> + 'b {
    namespace.dtos().enumerate().filter_map(move |(i, dto)| {
        if dto.name.is_empty() {
            Some(ValidationError::InvalidDtoName(
                namespace_id.clone().to_owned(),
                i,
            ))
        } else {
            None
        }
    })
}

pub fn dto_field_names<'a, 'b>(
    api: &'b Api<'a>,
    namespace: &'b Namespace<'a>,
    namespace_id: EntityId,
) -> impl Iterator<Item = ValidationError> + 'b {
    namespace
        .dtos()
        .flat_map(move |dto| field_names(api, dto.fields.iter(), namespace_id.child(dto.name)))
}

pub fn dto_field_types<'a, 'b>(
    api: &'b Api<'a>,
    namespace: &'b Namespace<'a>,
    namespace_id: EntityId,
) -> impl Iterator<Item = ValidationError> + 'b {
    namespace
        .dtos()
        .flat_map(move |dto| field_types(api, dto.fields.iter(), namespace_id.child(dto.name)))
}

pub fn rpc_names<'a, 'b>(
    _: &'b Api<'a>,
    namespace: &'b Namespace<'a>,
    namespace_id: EntityId,
) -> impl Iterator<Item = ValidationError> + 'b {
    namespace.rpcs().enumerate().filter_map(move |(i, rpc)| {
        if rpc.name.is_empty() {
            Some(ValidationError::InvalidRpcName(namespace_id.to_owned(), i))
        } else {
            None
        }
    })
}

pub fn rpc_param_names<'a, 'b>(
    api: &'b Api<'a>,
    namespace: &'b Namespace<'a>,
    namespace_id: EntityId,
) -> impl Iterator<Item = ValidationError> + 'b {
    namespace
        .rpcs()
        .flat_map(move |rpc| field_names(api, rpc.params.iter(), namespace_id.child(rpc.name)))
}

pub fn rpc_param_types<'a, 'b>(
    api: &'b Api<'a>,
    namespace: &'b Namespace<'a>,
    entity_id: EntityId,
) -> impl Iterator<Item = ValidationError> + 'b {
    namespace
        .rpcs()
        .flat_map(move |rpc| field_types(api, rpc.params.iter(), entity_id.child(rpc.name)))
}

pub fn rpc_return_types<'a, 'b>(
    api: &'b Api<'a>,
    namespace: &'b Namespace<'a>,
    namespace_id: EntityId,
) -> impl Iterator<Item = ValidationError> + 'b {
    namespace
        .rpcs()
        .filter_map(|rpc| rpc.return_type.as_ref().map(|ty| (rpc.name, ty)))
        .filter_map(move |(rpc_name, return_type)| {
            if find_type_relative(api, namespace_id.clone(), &return_type) {
                None
            } else {
                Some(ValidationError::InvalidRpcReturnType(
                    namespace_id.child(rpc_name).to_owned(),
                    return_type.to_owned(),
                ))
            }
        })
}

pub fn field_names<'a, 'b>(
    _: &'b Api<'a>,
    fields: impl Iterator<Item = &'b Field<'a>> + 'a + 'b,
    parent_entity_id: EntityId,
) -> impl Iterator<Item = ValidationError> + 'a + 'b {
    fields.enumerate().filter_map(move |(i, field)| {
        if field.name.is_empty() {
            Some(ValidationError::InvalidFieldName(
                parent_entity_id.to_owned(),
                i,
            ))
        } else {
            None
        }
    })
}

pub fn field_types<'a, 'b: 'a>(
    api: &'b Api<'a>,
    fields: impl Iterator<Item = &'b Field<'a>> + 'b,
    parent_entity_id: EntityId,
) -> impl Iterator<Item = ValidationError> + 'a + 'b {
    fields.enumerate().filter_map(move |(i, field)| {
        if find_type_relative(api, parent_entity_id.clone(), &field.ty) {
            None
        } else {
            Some(ValidationError::InvalidFieldType(
                parent_entity_id.clone(),
                field.name.to_string(),
                i,
                field.ty.to_owned(),
            ))
        }
    })
}

/// Find `find_ty` by walking up the namespace hierarchy in `api`, starting at `initial_namespace`.
fn find_type_relative(api: &Api, initial_namespace: EntityId, find_ty: &EntityId) -> bool {
    let mut iter = initial_namespace;
    loop {
        let namespace = api.find_namespace(&iter);
        match namespace {
            None => return false,
            Some(namespace) => {
                if namespace.find_dto(find_ty).is_some() {
                    return true;
                }
            }
        }
        iter = match iter.parent() {
            None => return false,
            Some(id) => id,
        }
    }
}

/// Calls the function `f` for each [Namespace] in the `api`. `f` will be passed the [Namespace]
/// currently being operated on and a [EntityId] to that namespace within the overall hierarchy.
///
/// `'a` is the lifetime of the [Api] bound.
/// `'b` is the lifetime of the [Builder::build] process.
pub(crate) fn recurse_api<'a, 'b, I, F>(api: &'b Api<'a>, f: F) -> Vec<ValidationError>
where
    'b: 'a,
    I: Iterator<Item = ValidationError>,
    F: Copy + Fn(&'b Api<'a>, &'b Namespace<'a>, EntityId) -> I,
{
    recurse_namespaces(api, api, EntityId::default(), f)
}

fn recurse_namespaces<'a, 'b, I, F>(
    api: &'b Api<'a>,
    namespace: &'b Namespace<'a>,
    entity_id: EntityId,
    f: F,
) -> Vec<ValidationError>
where
    'b: 'a,
    I: Iterator<Item = ValidationError>,
    F: Copy + Fn(&'b Api<'a>, &'b Namespace<'a>, EntityId) -> I,
{
    let child_errors = namespace
        .namespaces()
        .flat_map(|child| recurse_namespaces(api, child, entity_id.child(&child.name), f));

    child_errors
        .chain(f(api, namespace, entity_id.clone()))
        .collect_vec()
}

#[cfg(test)]
mod tests {
    // note: many validators tested via actual code paths in builder.

    // todo param type tests
    // todo return type tests

    use crate::model::validate::rpc_return_types;
    use crate::model::EntityId;
    use crate::test_util::executor::TestExecutor;
    use itertools::Itertools;

    #[test]
    fn test_rpc_return_types() {
        let mut exe = TestExecutor::new(
            r#"
            mod ns0 {
                mod ns1 {
                    mod ns2 {
                        fn rpc() -> other0::other1::dto1 {}
                        fn rpc() -> other0::dto2 {}
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

        let namespace_id = EntityId::new(["ns0", "ns1", "ns2"]);
        let initial_namespace = api.find_namespace(&namespace_id).unwrap();
        assert_eq!(
            rpc_return_types(&api, initial_namespace, namespace_id).collect_vec(),
            vec![]
        );
    }

    mod field_types {
        use itertools::Itertools;

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
                &EntityId::new(["dto0"]),
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
                &EntityId::new(["ns0", "ns1", "dto0"]),
            );
        }

        #[test]
        fn relative_path_local() {
            run_test(
                r#"
                mod ns0 {
                    mod ns1 {
                        struct dto0 {
                            field: dto1,
                        }
                        struct dto1 {}
                    }
                }
                "#,
                &EntityId::new(["ns0", "ns1", "dto0"]),
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
                &EntityId::new(["ns0", "ns1", "dto0"]),
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
                &EntityId::new(["ns0", "dto0"]),
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
                &EntityId::new(["ns0", "ns1", "dto0"]),
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
                &EntityId::new(["ns0", "ns1", "ns2", "dto0"]),
            );
        }

        fn run_test(input_data: &str, source_dto: &EntityId) {
            let mut exe = TestExecutor::new(input_data);
            let api = exe.api();

            assert_eq!(
                field_types(
                    &api,
                    api.find_dto(source_dto)
                        .expect("couldn't find source dto")
                        .fields
                        .iter(),
                    source_dto.parent().expect("dto has no parent"),
                )
                .collect_vec(),
                vec![]
            );
        }
    }
}
