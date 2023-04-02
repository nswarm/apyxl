use anyhow::Result;
use itertools::Itertools;
use log::info;
use thiserror::Error;

use crate::model::{Api, Namespace, TypeRef, UNDEFINED_NAMESPACE};

/// Helper struct for parsing [Namespace]s spread across multiple chunks.
/// After all desired [Namespace]s are merged, the [Builder] can be finalized via [Builder::build] which will
/// perform validation across the entire [Api].
pub struct Builder<'a> {
    api: Api<'a>,
    namespace_stack: Vec<&'a str>,
}

#[derive(Error, Debug, Eq, PartialEq)]
pub enum ValidationError<'a> {
    #[error(
        "Invalid namespace found at path {0:?}. Only the root namespace can be named {}.",
        UNDEFINED_NAMESPACE
    )]
    InvalidNamespaceName(TypeRef<'a>),

    #[error("Invalid DTO name within namespace '{0:?}', RPC #{1}. DTO names cannot be empty.")]
    InvalidDtoName(TypeRef<'a>, usize),

    #[error("Invalid field name within DTO '{0:?}', field #{1}. Field names cannot be empty.")]
    InvalidDtoFieldName(TypeRef<'a>, usize),

    #[error("Invalid type for field '{0:?}::{1}'. Type '{2:?}' must be a valid DTO in the API.")]
    InvalidDtoFieldType(TypeRef<'a>, &'a str, TypeRef<'a>),

    #[error("Invalid RPC name within namespace '{0:?}', RPC #{1}. RPC names cannot be empty.")]
    InvalidRpcName(TypeRef<'a>, usize),

    #[error(
        "Invalid field name within DTO '{0:?}', parameter #{1}. Parameter names cannot be empty."
    )]
    InvalidRpcParameterName(TypeRef<'a>, usize),

    #[error("Invalid type for param in RPC '{0:?}', parameter '{1}'. Type '{2:?}' must be a valid DTO in the API.")]
    InvalidRpcParameterType(TypeRef<'a>, &'a str, TypeRef<'a>),

    #[error("Invalid return type for RPC {0:?}. Type '{1:?}' must be a valid DTO in the API.")]
    InvalidRpcReturnType(TypeRef<'a>, TypeRef<'a>),

    #[error("Duplicate DTO definition: {0:?}")]
    DuplicateDto(TypeRef<'a>),

    #[error("Duplicate RPC definition: {0:?}")]
    DuplicateRpc(TypeRef<'a>),
}

impl Default for Builder<'_> {
    fn default() -> Self {
        Self {
            api: Api {
                name: UNDEFINED_NAMESPACE,
                ..Default::default()
            },
            namespace_stack: Default::default(),
        }
    }
}

impl<'a> Builder<'a> {
    /// Merge `namespace` with into the builder's [Api].
    ///
    /// If the `name` of the `namespace` is [UNDEFINED_NAMESPACE] it will be merged with the
    /// current builder namespace. Otherwise it will be added as a new namespace underneath the
    /// current builder namespace.
    pub fn merge(&mut self, namespace: Namespace<'a>) {
        if namespace.name == UNDEFINED_NAMESPACE {
            self.current_namespace_mut().merge(namespace)
        } else {
            self.current_namespace_mut().add_namespace(namespace)
        }
    }

    /// Add `namespace` to the current namespace stack of the Builder. Any [Api]s merged will be
    /// nested within the full namespace specified by the stack.
    pub fn enter_namespace(&mut self, name: &'a str) {
        if self.current_namespace().namespace(name).is_none() {
            self.current_namespace_mut().add_namespace(Namespace {
                name,
                ..Default::default()
            });
        }
        self.namespace_stack.push(name);
    }

    /// Remove the most recently-added namespace from the stack.
    pub fn exit_namespace(&mut self) {
        self.namespace_stack.pop();
    }

    /// Finalize and validate the API.
    /// - Dedupes namespaces recursively.
    /// - Errors for [Dto]s and [Rpc]s with identical paths (aka duplicate definitions).
    /// - Errors for TypeRefs with missing types not specified in list of assumed types.
    pub fn build(mut self) -> Result<Api<'a>, Vec<ValidationError<'a>>> {
        dedupe_namespace_children(&mut self.api);

        let mut errors = [
            validate_namespace_names(&self.api, &TypeRef::default()),
            validate_dtos(&self.api, &TypeRef::default()),
            validate_rpcs(&self.api, &TypeRef::default()),
            validate_no_duplicates(&self.api, &TypeRef::default()),
        ]
        .into_iter()
        .flatten()
        .collect_vec();
        if errors.is_empty() {
            Ok(self.api)
        } else {
            Err(errors)
        }
    }

    fn current_namespace(&self) -> &Namespace<'a> {
        self.api.find_namespace(&self.namespace_stack.as_slice().into())
            .expect("enter_namespace must always create the namespace if it does not exist, which will guarantee this never fails")
    }

    fn current_namespace_mut(&mut self) -> &mut Namespace<'a> {
        self.api.find_namespace_mut(&self.namespace_stack.as_slice().into())
            .expect("enter_namespace must always create the namespace if it does not exist, which will guarantee this never fails")
    }
}

fn dedupe_namespace_children(namespace: &mut Namespace) {
    info!("deduping namespaces...");
    namespace
        .take_namespaces()
        .into_iter()
        .sorted_unstable_by_key(|ns| ns.name)
        .coalesce(|mut lhs, rhs| {
            if rhs.name == lhs.name {
                lhs.merge(rhs);
                Ok(lhs)
            } else {
                Err((lhs, rhs))
            }
        })
        .for_each(|mut ns| {
            dedupe_namespace_children(&mut ns);
            namespace.add_namespace(ns)
        });
}

fn validate_namespace_names<'a>(
    namespace: &Namespace<'a>,
    parent_ref: &TypeRef<'a>,
) -> Vec<ValidationError<'a>> {
    info!("validating namespace names...");
    let mut errors = Vec::new();
    for child in namespace.namespaces() {
        let type_ref = parent_ref.child(child.name);
        if child.name == UNDEFINED_NAMESPACE {
            errors.push(ValidationError::InvalidNamespaceName(type_ref));
            continue;
        }
        let mut child_errors = validate_namespace_names(child, &type_ref);
        errors.append(&mut child_errors);
    }
    errors
}

fn validate_no_duplicates<'a>(
    namespace: &Namespace<'a>,
    type_ref: &TypeRef<'a>,
) -> Vec<ValidationError<'a>> {
    info!("validating no duplicate definitions...");
    let mut errors = Vec::new();
    for child in namespace.namespaces() {
        let mut child_errors = validate_no_duplicates(child, &type_ref.child(child.name));
        errors.append(&mut child_errors);
    }
    for dto in namespace.dtos().duplicates_by(|dto| dto.name) {
        errors.push(ValidationError::DuplicateDto(type_ref.child(dto.name)))
    }
    for rpc in namespace.rpcs().duplicates_by(|rpc| rpc.name) {
        errors.push(ValidationError::DuplicateRpc(type_ref.child(rpc.name)))
    }
    errors
}

fn validate_dtos<'a>(
    namespace: &Namespace<'a>,
    type_ref: &TypeRef<'a>,
) -> Vec<ValidationError<'a>> {
    info!("validating type refs...");
    vec![]
}

fn validate_rpcs<'a>(
    namespace: &Namespace<'a>,
    type_ref: &TypeRef<'a>,
) -> Vec<ValidationError<'a>> {
    info!("validating type refs...");
    vec![]
}

#[cfg(test)]
mod tests {
    mod namespace {
        use crate::model::Builder;

        #[test]
        fn multiple_enter_exit() {
            let mut builder = Builder::default();
            builder.enter_namespace("a");
            builder.enter_namespace("b");
            builder.exit_namespace();
            builder.enter_namespace("c");
            builder.enter_namespace("d");
            builder.exit_namespace();
            assert_eq!(builder.namespace_stack, vec!["a", "c"]);
        }

        #[test]
        fn exit_on_empty_does_not_explode() {
            let mut builder = Builder::default();
            builder.exit_namespace();
            assert_eq!(builder.namespace_stack, Vec::<&str>::default())
        }
    }

    mod merge {
        use crate::model::{Dto, Namespace, NamespaceChild};

        mod no_current_namespace {
            use crate::model::api::builder::tests::merge::{
                test_child_dto, test_child_namespace, test_named_namespace, test_namespace,
                NS_NAMES,
            };
            use crate::model::{Builder, Namespace, NamespaceChild, UNDEFINED_NAMESPACE};

            #[test]
            fn name_is_root() {
                let mut builder = Builder::default();
                builder.merge(test_named_namespace(UNDEFINED_NAMESPACE, 1));
                assert_eq!(builder.api.name, UNDEFINED_NAMESPACE, "no change root name");
                assert_eq!(builder.api.children, vec![test_child_dto(1)]);
            }

            #[test]
            fn name_is_empty() {
                let mut builder = Builder::default();
                // Anonymous namespace same as "new".
                builder.merge(test_named_namespace("", 1));
                assert_eq!(
                    builder.api.children,
                    vec![NamespaceChild::Namespace(test_named_namespace("", 1))]
                );
            }

            #[test]
            fn name_is_new() {
                let mut builder = Builder::default();
                builder.api.children.push(test_child_namespace(1));
                builder.merge(test_namespace(2));
                assert_eq!(
                    builder.api.children,
                    vec![test_child_namespace(1), test_child_namespace(2)]
                );
            }

            #[test]
            fn name_is_existing() {
                let mut builder = Builder::default();
                builder.api.children.push(test_child_namespace(1));
                builder.merge(test_named_namespace(NS_NAMES[1], 2));
                assert_eq!(
                    builder.api.children,
                    vec![
                        // Duplicates preserved.
                        test_child_namespace(1),
                        NamespaceChild::Namespace(Namespace {
                            name: NS_NAMES[1],
                            children: vec![test_child_dto(2)],
                        })
                    ]
                );
            }
        }

        mod has_current_namespace {
            use crate::model::api::builder::tests::merge::{
                test_child_dto, test_child_namespace, test_named_namespace, test_namespace,
            };
            use crate::model::{Builder, Namespace, NamespaceChild, UNDEFINED_NAMESPACE};

            #[test]
            fn name_is_root() {
                let mut builder = test_builder();
                builder.merge(test_named_namespace(UNDEFINED_NAMESPACE, 1));
                assert_eq!(builder.api.name, UNDEFINED_NAMESPACE, "no change root name");

                let mut expected = current_namespace();
                expected.children.push(test_child_dto(1));
                assert_eq!(
                    builder.api.children,
                    vec![NamespaceChild::Namespace(expected)]
                );
            }

            #[test]
            fn name_is_empty() {
                let mut builder = test_builder();
                builder.merge(test_named_namespace("", 1));

                // Anonymous namespace same as "new".
                let mut expected = current_namespace();
                expected
                    .children
                    .push(NamespaceChild::Namespace(test_named_namespace("", 1)));
                assert_eq!(
                    builder.api.children,
                    vec![NamespaceChild::Namespace(expected)]
                );
            }

            #[test]
            fn name_is_new() {
                let mut builder = test_builder();
                builder.merge(test_namespace(2));

                let mut expected = current_namespace();
                expected.children.push(test_child_namespace(2));
                assert_eq!(
                    builder.api.children,
                    vec![NamespaceChild::Namespace(expected)]
                );
            }

            #[test]
            fn name_is_existing() {
                let mut builder = test_builder();
                if let NamespaceChild::Namespace(ns) = builder.api.children.get_mut(0).unwrap() {
                    ns.children.push(test_child_namespace(2));
                }
                builder.merge(test_namespace(2));

                let mut expected = current_namespace();
                // Duplicates preserved.
                expected.children.push(test_child_namespace(2));
                expected.children.push(test_child_namespace(2));
                assert_eq!(
                    builder.api.children,
                    vec![NamespaceChild::Namespace(expected)],
                );
            }

            const CURRENT_NAMESPACE: &str = "current";
            fn current_namespace() -> Namespace<'static> {
                test_named_namespace(CURRENT_NAMESPACE, 4)
            }

            fn test_builder() -> Builder<'static> {
                let mut builder = Builder::default();
                builder.api.add_namespace(current_namespace());
                builder.enter_namespace(CURRENT_NAMESPACE);
                builder
            }
        }

        fn test_child_namespace(i: usize) -> NamespaceChild<'static> {
            NamespaceChild::Namespace(test_namespace(i))
        }

        const NS_NAMES: &[&str] = &["ns0", "ns1", "ns2", "ns3", "ns4"];
        fn test_namespace(i: usize) -> Namespace<'static> {
            test_named_namespace(NS_NAMES[i], i)
        }

        fn test_named_namespace(name: &'static str, i: usize) -> Namespace<'static> {
            Namespace {
                name,
                children: vec![test_child_dto(i)],
            }
        }

        fn test_child_dto(i: usize) -> NamespaceChild<'static> {
            NamespaceChild::Dto(test_dto(i))
        }

        const DTO_NAMES: &[&str] = &["DtoName0", "DtoName1", "DtoName2", "DtoName3", "DtoName4"];
        fn test_dto(i: usize) -> Dto<'static> {
            Dto {
                name: DTO_NAMES[i],
                fields: vec![],
            }
        }
    }

    mod build {
        use crate::input;
        use crate::model::api::builder::ValidationError;
        use crate::model::tests::test_api;
        use crate::model::{Api, Builder};

        mod dedupe_namespaces {
            use crate::input;
            use crate::model::api::builder::tests::build::build_from_input;

            #[test]
            fn within_root() {
                let mut input = input::Buffer::new(
                    r#"
                    mod ns2 {}
                    mod ns1 {}
                    mod ns1 {}
                    mod ns3 {}
                    mod ns2 {}
                    mod ns1 {}
                "#,
                );
                let api = build_from_input(&mut input).unwrap();

                assert_eq!(api.namespaces().count(), 3);
                assert!(api.namespace("ns1").is_some());
                assert!(api.namespace("ns2").is_some());
                assert!(api.namespace("ns3").is_some());
            }

            #[test]
            fn within_sub_namespace() {
                let mut input = input::Buffer::new(
                    r#"
                    mod ns {
                        mod ns2 {}
                        mod ns1 {}
                        mod ns1 {}
                        mod ns3 {}
                        mod ns2 {}
                        mod ns1 {}
                    }
                "#,
                );
                let api = build_from_input(&mut input).unwrap();

                assert_eq!(api.namespaces().count(), 1);
                assert!(api.namespace("ns").is_some());

                let nested = api.namespace("ns").unwrap();
                assert_eq!(nested.namespaces().count(), 3);
                assert!(nested.namespace("ns1").is_some());
                assert!(nested.namespace("ns2").is_some());
                assert!(nested.namespace("ns3").is_some());
            }

            #[test]
            fn across_sub_namespaces_with_same_name() {
                let mut input = input::Buffer::new(
                    r#"
                    mod ns {
                        mod ns1 {}
                        mod ns2 {}
                        mod ns3 {}
                    }
                    mod ns {
                        mod ns1 {}
                        mod ns2 {}
                        mod ns3 {}
                    }
                "#,
                );
                let api = build_from_input(&mut input).unwrap();

                assert_eq!(api.namespaces().count(), 1);
                let nested = api.namespace("ns").unwrap();
                assert_eq!(nested.namespaces().count(), 3);
                assert!(nested.namespace("ns1").is_some());
                assert!(nested.namespace("ns2").is_some());
                assert!(nested.namespace("ns3").is_some());
            }
        }

        mod validate_duplicates {
            use crate::input;
            use crate::model::api::builder::tests::build::{
                assert_contains_error, build_from_input,
            };
            use crate::model::api::builder::ValidationError;

            #[test]
            fn dtos() {
                let mut input = input::Buffer::new(
                    r#"
                    mod ns {
                        struct dto {}
                        struct dto {}
                    }
                "#,
                );
                let result = build_from_input(&mut input);
                assert_contains_error(&result, ValidationError::DuplicateDto(["ns", "dto"].into()));
            }

            #[test]
            fn rpcs() {
                let mut input = input::Buffer::new(
                    r#"
                    mod ns {
                        fn rpc() {}
                        fn rpc() {}
                    }
                "#,
                );
                let result = build_from_input(&mut input);
                assert_contains_error(&result, ValidationError::DuplicateRpc(["ns", "rpc"].into()));
            }

            #[test]
            fn rpc_dto_with_same_name_ok() {
                let mut input = input::Buffer::new(
                    r#"
                    mod ns {
                        fn thing() {}
                        struct thing {}
                    }
                "#,
                );
                let result = build_from_input(&mut input);
                assert!(result.is_ok());
            }
        }

        mod validate_dto {
            use crate::input;
            use crate::model::api::builder::tests::build::{assert_contains_error, test_builder};
            use crate::model::api::builder::ValidationError;
            use crate::model::TypeRef;

            #[test]
            fn name_empty() {
                let mut input = input::Buffer::new(
                    r#"
                    mod ns {
                        struct dto1 {}
                        struct dto2 {}
                        struct dto3 {}
                    }
                "#,
                );
                let mut builder = test_builder(&mut input);
                builder
                    .api
                    .find_dto_mut(&["ns", "dto3"].into())
                    .unwrap()
                    .name = "";

                let result = builder.build();
                let expected_type_ref = TypeRef::from(["ns"]);
                let expected_position = 3;
                assert_contains_error(
                    &result,
                    ValidationError::InvalidDtoName(expected_type_ref, expected_position),
                );
            }

            #[test]
            fn field_name_empty() {
                let mut input = input::Buffer::new(
                    r#"
                    mod ns {
                        struct dto {
                            field1: bool,
                            field2: bool,
                            field3: bool,
                        }
                    }"#,
                );
                let mut builder = test_builder(&mut input);
                builder
                    .api
                    .find_dto_mut(&["ns", "dto"].into())
                    .unwrap()
                    .field_mut("field2")
                    .unwrap()
                    .name = "";

                let result = builder.build();
                let expected_type_ref = TypeRef::from(["ns", "dto"]);
                let expected_position = 2;
                assert_contains_error(
                    &result,
                    ValidationError::InvalidDtoFieldName(expected_type_ref, expected_position),
                );
            }

            #[test]
            fn field_type_empty() {
                let mut input = input::Buffer::new(
                    r#"
                    mod ns {
                        struct dto {
                            field1: bool,
                            field2: bool,
                            field3: bool,
                        }
                    }"#,
                );
                let mut builder = test_builder(&mut input);
                builder
                    .api
                    .find_dto_mut(&["ns", "dto"].into())
                    .unwrap()
                    .field_mut("field2")
                    .unwrap()
                    .ty = TypeRef::default();

                let result = builder.build();
                let expected_type_ref = TypeRef::from(["ns"]);
                let expected_position = 2;
                assert_contains_error(
                    &result,
                    ValidationError::InvalidDtoFieldType(
                        expected_type_ref,
                        "field2",
                        TypeRef::default(),
                    ),
                );
            }

            #[test]
            fn field_type_valid_linkage() {
                todo!("nyi");
            }
        }

        mod validate_rpc {
            use crate::input;
            use crate::model::api::builder::tests::build::{assert_contains_error, test_builder};
            use crate::model::api::builder::ValidationError;
            use crate::model::TypeRef;

            #[test]
            fn name_empty() {
                let mut input = input::Buffer::new(
                    r#"
                    mod ns {
                        fn rpc1() {}
                        fn rpc2() {}
                        fn rpc3() {}
                    }
                "#,
                );
                let mut builder = test_builder(&mut input);
                builder
                    .api
                    .find_rpc_mut(&["ns", "rpc3"].into())
                    .unwrap()
                    .name = "";

                let result = builder.build();
                let expected_type_ref = TypeRef::from(["ns"]);
                let expected_position = 3;
                assert_contains_error(
                    &result,
                    ValidationError::InvalidRpcName(expected_type_ref, expected_position),
                );
            }

            #[test]
            fn param_name_empty() {
                let mut input = input::Buffer::new(
                    r#"
                    mod ns {
                        fn rpc(param1: bool, param2: bool, param3: bool) {}
                    "#,
                );
                let mut builder = test_builder(&mut input);
                builder
                    .api
                    .find_rpc_mut(&["ns", "rpc"].into())
                    .unwrap()
                    .param_mut("param2")
                    .unwrap()
                    .name = "";

                let result = builder.build();
                let expected_type_ref = TypeRef::from(["ns", "rpc"]);
                let expected_position = 2;
                assert_contains_error(
                    &result,
                    ValidationError::InvalidRpcParameterName(expected_type_ref, expected_position),
                );
            }

            #[test]
            fn param_type_empty() {
                let mut input = input::Buffer::new(
                    r#"
                    mod ns {
                        fn rpc(param1: bool, param2: bool, param3: bool) {}
                    "#,
                );
                let mut builder = test_builder(&mut input);
                builder
                    .api
                    .find_rpc_mut(&["rpc"].into())
                    .unwrap()
                    .param_mut("param2")
                    .unwrap()
                    .ty = TypeRef::default();

                let result = builder.build();
                let expected_type_ref = TypeRef::from(["ns", "rpc"]);
                let expected_position = 2;
                assert_contains_error(
                    &result,
                    ValidationError::InvalidRpcParameterType(
                        expected_type_ref,
                        "param2",
                        TypeRef::default(),
                    ),
                );
            }

            #[test]
            fn param_type_valid_linkage() {
                todo!("nyi");
            }

            #[test]
            fn return_type_valid_linkage() {
                todo!("nyi");
            }
        }

        mod validate_namespace {
            use crate::input;
            use crate::model::api::builder::tests::build::{
                assert_contains_error, build_from_input,
            };
            use crate::model::api::builder::ValidationError;
            use crate::model::{Builder, UNDEFINED_NAMESPACE};

            #[test]
            fn root_namespace_undefined_allowed() {
                let mut builder = Builder::default();
                assert!(builder.build().is_ok());
            }

            #[test]
            fn name_within_root_not_undefined() {
                let mut input =
                    input::Buffer::new("mod zzzz {}".replace("zzzz", UNDEFINED_NAMESPACE));
                assert_contains_error(
                    &build_from_input(&mut input),
                    ValidationError::InvalidNamespaceName([UNDEFINED_NAMESPACE].into()),
                );
            }

            #[test]
            fn name_below_root_not_undefined() {
                let mut input = input::Buffer::new(
                    r#"
                    mod ns {
                        mod zzzz {}
                    }"#
                    .replace("zzzz", UNDEFINED_NAMESPACE),
                );
                assert_contains_error(
                    &build_from_input(&mut input),
                    ValidationError::InvalidNamespaceName(["ns", UNDEFINED_NAMESPACE].into()),
                );
            }
        }

        fn build_from_input(input: &mut input::Buffer) -> Result<Api, Vec<ValidationError>> {
            test_builder(input).build()
        }

        fn test_builder(input: &mut input::Buffer) -> Builder {
            Builder {
                api: test_api(input),
                ..Default::default()
            }
        }

        fn assert_contains_error(
            build_result: &Result<Api, Vec<ValidationError>>,
            error: ValidationError,
        ) {
            let errors = build_result
                .as_ref()
                .map(|_| "...but it passed!")
                .expect_err("expected Builder::build to fail");
            assert!(errors.contains(&error), "actual: {:?}", errors);
        }
    }
}
