use anyhow::Result;
use itertools::Itertools;
use log::info;
use thiserror::Error;

use crate::model::{Api, Namespace, NamespaceChild, TypeRef, UNDEFINED_NAMESPACE};

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

    #[error("Invalid DTO name within namespace {0:?}, RPC #{1}. DTO names cannot be empty.")]
    InvalidDtoName(TypeRef<'a>, usize),

    #[error("Invalid field name within DTO {0:?}, field #{1}. Field names cannot be empty.")]
    InvalidDtoFieldName(TypeRef<'a>, &'a str),

    #[error("Invalid type for field {0:?}::{1}. Type '{2:?}' must be a valid DTO in the API.")]
    InvalidDtoFieldType(TypeRef<'a>, &'a str, TypeRef<'a>),

    #[error("Invalid RPC name within namespace {0:?}, RPC #{1}. RPC names cannot be empty.")]
    InvalidRpcName(TypeRef<'a>, usize),

    #[error(
        "Invalid field name within DTO {0:?}, parameter #{1}. Parameter names cannot be empty."
    )]
    InvalidRpcParameterName(TypeRef<'a>, &'a str),

    #[error("Invalid type for param in RPC {0:?}::{1}, parameter {2}. Type '{3:?}' must be a valid DTO in the API.")]
    InvalidRpcParameterType(TypeRef<'a>, &'a str, &'a str, TypeRef<'a>),

    #[error(
        "Invalid return type for RPC {0:?}::{1}. Type '{2:?}' must be a valid DTO in the API."
    )]
    InvalidRpcReturnType(TypeRef<'a>, &'a str, TypeRef<'a>),

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
        validate_namespace_names(&self.api, &TypeRef::default())?;
        validate_no_duplicates(&self.api, &TypeRef::default())?;
        validate_type_refs(&self.api, &TypeRef::default())?;
        Ok(self.api)
    }

    fn current_namespace(&self) -> &Namespace<'a> {
        self.api.find_namespace(&TypeRef::from(self.namespace_stack.as_slice()))
            .expect("enter_namespace must always create the namespace if it does not exist, which will guarantee this never fails")
    }

    fn current_namespace_mut(&mut self) -> &mut Namespace<'a> {
        self.api.find_namespace_mut(&TypeRef::from(self.namespace_stack.as_slice()))
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
    parent: &TypeRef<'a>,
) -> Result<(), Vec<ValidationError<'a>>> {
    info!("validating namespace names...");
    let mut errors = Vec::new();
    for namespace in namespace.namespaces() {
        let type_ref = parent.child(namespace.name);
        if namespace.name == UNDEFINED_NAMESPACE {
            errors.push(ValidationError::InvalidNamespaceName(type_ref));
            continue;
        }
        let result = validate_namespace_names(namespace, &type_ref);
        append_errors(&mut errors, result)
    }
    errors_to_result(errors)
}

fn validate_no_duplicates<'a>(
    namespace: &Namespace<'a>,
    type_ref: &TypeRef<'a>,
) -> Result<(), Vec<ValidationError<'a>>> {
    info!("validating no duplicate definitions...");
    Ok(())
}

fn validate_type_refs<'a>(
    namespace: &Namespace<'a>,
    type_ref: &TypeRef<'a>,
) -> Result<(), Vec<ValidationError<'a>>> {
    info!("validating type refs...");
    Ok(())
}

fn append_errors<'a>(
    errors: &mut Vec<ValidationError<'a>>,
    result: Result<(), Vec<ValidationError<'a>>>,
) {
    if let Err(mut child_errors) = result {
        errors.append(&mut child_errors)
    }
}

fn errors_to_result(errors: Vec<ValidationError>) -> Result<(), Vec<ValidationError>> {
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
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
        use crate::model::api::builder::ValidationError;
        use crate::model::tests::{complex_api, complex_namespace, test_dto};
        use crate::model::{Api, Builder, TypeRef};

        mod dedupe_namespaces {
            use crate::model::tests::test_namespace;
            use crate::model::Builder;

            #[test]
            fn within_root() {
                let mut builder = Builder::default();
                builder.api.add_namespace(test_namespace(2));
                builder.api.add_namespace(test_namespace(1));
                builder.api.add_namespace(test_namespace(1));
                builder.api.add_namespace(test_namespace(3));
                builder.api.add_namespace(test_namespace(2));
                builder.api.add_namespace(test_namespace(1));
                let api = builder.build().expect("build failed");
                assert_eq!(api.namespaces().count(), 3);
                assert!(api.namespace(test_namespace(1).name).is_some());
                assert!(api.namespace(test_namespace(2).name).is_some());
                assert!(api.namespace(test_namespace(3).name).is_some());
            }

            #[test]
            fn within_sub_namespace() {
                let mut builder = Builder::default();
                let mut nested_namespace = test_namespace(4);
                nested_namespace.add_namespace(test_namespace(2));
                nested_namespace.add_namespace(test_namespace(1));
                nested_namespace.add_namespace(test_namespace(1));
                nested_namespace.add_namespace(test_namespace(3));
                nested_namespace.add_namespace(test_namespace(2));
                nested_namespace.add_namespace(test_namespace(1));
                builder.api.add_namespace(nested_namespace);

                let api = builder.build().expect("build failed");
                assert_eq!(api.namespaces().count(), 1);
                assert!(api.namespace(test_namespace(4).name).is_some());

                let nested = api.namespace(test_namespace(4).name).unwrap();
                assert_eq!(nested.namespaces().count(), 3);
                assert!(nested.namespace(test_namespace(1).name).is_some());
                assert!(nested.namespace(test_namespace(2).name).is_some());
                assert!(nested.namespace(test_namespace(3).name).is_some());
            }

            #[test]
            fn across_sub_namespaces_with_same_name() {
                let mut builder = Builder::default();

                for _ in 0..2 {
                    let mut nested_namespace = test_namespace(1);
                    nested_namespace.add_namespace(test_namespace(2));
                    builder.api.add_namespace(nested_namespace);
                }

                let api = builder.build().expect("build failed");
                assert_eq!(api.namespaces().count(), 1);
                let nested = api.namespace(test_namespace(1).name).unwrap();
                assert_eq!(nested.namespaces().count(), 1);
            }
        }

        mod validate_dto {
            #[test]
            fn name_not_empty() {
                todo!("nyi");
            }

            #[test]
            fn field_name_not_empty() {
                todo!("nyi");
            }

            #[test]
            fn field_type_not_empty() {
                todo!("nyi");
            }

            #[test]
            fn field_type_valid_linkage() {
                todo!("nyi");
            }
        }

        mod validate_rpc {
            #[test]
            fn name_not_empty() {
                todo!("nyi");
            }

            #[test]
            fn param_name_not_empty() {
                todo!("nyi");
            }

            #[test]
            fn param_type_not_empty() {
                todo!("nyi");
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
            use crate::model::api::builder::tests::build::{assert_contains_error, test_builder};
            use crate::model::api::builder::ValidationError;
            use crate::model::tests::{complex_namespace, test_namespace};
            use crate::model::{TypeRef, UNDEFINED_NAMESPACE};

            #[test]
            fn root_namespace_undefined_allowed() {
                let builder = test_builder();
                assert!(builder.build().is_ok());
            }

            #[test]
            fn name_within_root_not_undefined() {
                let mut builder = test_builder();
                builder
                    .api
                    .namespace_mut(complex_namespace(1).name)
                    .unwrap()
                    .name = UNDEFINED_NAMESPACE;
                assert_contains_error(
                    &builder.build(),
                    ValidationError::InvalidNamespaceName(TypeRef::from(
                        [UNDEFINED_NAMESPACE].as_slice(),
                    )),
                );
            }

            #[test]
            fn name_below_root_not_undefined() {
                let mut builder = test_builder();
                builder
                    .api
                    .namespace_mut(complex_namespace(1).name)
                    .unwrap()
                    .namespace_mut(test_namespace(3).name)
                    .unwrap()
                    .name = UNDEFINED_NAMESPACE;
                assert_contains_error(
                    &builder.build(),
                    ValidationError::InvalidNamespaceName(TypeRef::from(
                        [complex_namespace(1).name, UNDEFINED_NAMESPACE].as_slice(),
                    )),
                );
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

        fn test_builder() -> Builder<'static> {
            Builder {
                api: complex_api(),
                ..Default::default()
            }
        }

        fn valid_dto_ref() -> TypeRef<'static> {
            TypeRef::from([complex_namespace(1).name, test_dto(3).name].as_slice())
        }

        fn invalid_dto_ref() -> TypeRef<'static> {
            TypeRef::from([complex_namespace(1).name, "i_dont_exist"].as_slice())
        }
    }
}
