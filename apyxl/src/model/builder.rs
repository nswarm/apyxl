use anyhow::Result;
use itertools::Itertools;

use crate::input;
use crate::model::api::validate;
use crate::model::{
    chunk, Api, EntityId, Metadata, Model, Namespace, ValidationError, UNDEFINED_NAMESPACE,
};

/// Helper struct made for parsing [Api]s spread across multiple [Chunk]s. Tracks [Metadata]
/// associated with entities in the [Api]s.
///
/// After all desired [Api]s are merged, the [Builder] can be finalized via [Builder::build] which will
/// perform validation across the entire [Api] and return the final [Model].
pub struct Builder<'a> {
    api: Api<'a>,
    namespace_stack: Vec<&'a str>,
    metadata: Metadata<'a>,
}

impl Default for Builder<'_> {
    fn default() -> Self {
        Self {
            api: Api {
                name: UNDEFINED_NAMESPACE,
                ..Default::default()
            },
            namespace_stack: Default::default(),
            metadata: Default::default(),
        }
    }
}

impl<'a> Builder<'a> {
    /// Merge `namespace` into the builder's [Api].
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

    /// A version of [Builder::merge] that does the following in addition to the [Api] merge:
    /// - Adds the appropriate [chunk::Metadata] to the builder's [Metadata].
    /// - Applies the [chunk::Attribute] to all entities in the namespace recursively.
    pub fn merge_from_chunk(&mut self, mut namespace: Namespace<'a>, chunk: &input::Chunk) {
        if let Some(relative_file_path) = &chunk.relative_file_path {
            let root_namespace = self.current_namespace_id();
            self.metadata_mut().chunks.push(chunk::Metadata {
                root_namespace,
                relative_file_path: chunk.relative_file_path.clone(),
            });
            namespace.apply_attr_to_children_recursively(|attr| {
                attr.chunk
                    .get_or_insert(chunk::Attribute::default())
                    .relative_file_paths
                    .push(relative_file_path.clone())
            });
        }

        self.merge(namespace);
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

    /// Finalize and validate the model.
    /// - Dedupes namespaces recursively.
    /// - Errors for [Dto]s or [Rpc]s with empty names.
    /// - Errors for [Dto]s with identical paths (aka duplicate definitions).
    /// - Errors for [Rpc]s with identical paths (aka duplicate definitions).
    /// - Errors for [EntityId]s with missing types.
    pub fn build(mut self) -> Result<Model<'a>, Vec<ValidationError<'a>>> {
        dedupe_namespace_children(&mut self.api);

        let errors = [
            recurse_api(&self.api, validate::namespace_names),
            recurse_api(&self.api, validate::dto_names),
            recurse_api(&self.api, validate::dto_field_names),
            recurse_api(&self.api, validate::dto_field_types),
            recurse_api(&self.api, validate::rpc_names),
            recurse_api(&self.api, validate::rpc_param_names),
            recurse_api(&self.api, validate::rpc_param_types),
            recurse_api(&self.api, validate::rpc_return_types),
            recurse_api(&self.api, validate::no_duplicate_dtos),
            recurse_api(&self.api, validate::no_duplicate_rpcs),
        ]
        .into_iter()
        .flatten()
        .collect_vec();

        if errors.is_empty() {
            Ok(Model {
                api: self.api,
                metadata: Metadata::default(),
            })
        } else {
            Err(errors)
        }
    }

    pub fn metadata(&self) -> &Metadata<'_> {
        &self.metadata
    }

    pub fn metadata_mut(&mut self) -> &mut Metadata<'a> {
        &mut self.metadata
    }

    pub fn current_namespace_id(&self) -> EntityId<'a> {
        self.namespace_stack.clone().into()
    }

    pub fn current_namespace(&self) -> &Namespace<'a> {
        self.api.find_namespace(&self.namespace_stack.as_slice().into())
            .expect("enter_namespace must always create the namespace if it does not exist, which will guarantee this never fails")
    }

    pub fn current_namespace_mut(&mut self) -> &mut Namespace<'a> {
        self.api.find_namespace_mut(&self.namespace_stack.as_slice().into())
            .expect("enter_namespace must always create the namespace if it does not exist, which will guarantee this never fails")
    }

    #[cfg(test)]
    pub fn into_api(self) -> Api<'a> {
        self.api
    }
}

fn dedupe_namespace_children(namespace: &mut Namespace) {
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

/// Calls the function `f` for each [Namespace] in the `api`. `f` will be passed the [Namespace]
/// currently being operated on and a [EntityId] to that namespace within the overall hierarchy.
///
/// `'a` is the lifetime of the [Api] bound.
/// `'b` is the lifetime of the [Builder::build] process.
fn recurse_api<'a, 'b, I, F>(api: &'b Api<'a>, f: F) -> Vec<ValidationError<'a>>
where
    I: Iterator<Item = ValidationError<'a>>,
    F: Copy + Fn(&'b Api<'a>, &'b Namespace<'a>, EntityId<'a>) -> I,
{
    recurse_namespaces(api, api, EntityId::default(), f)
}

fn recurse_namespaces<'a, 'b, I, F>(
    api: &'b Api<'a>,
    namespace: &'b Namespace<'a>,
    entity_id: EntityId<'a>,
    f: F,
) -> Vec<ValidationError<'a>>
where
    I: Iterator<Item = ValidationError<'a>>,
    F: Copy + Fn(&'b Api<'a>, &'b Namespace<'a>, EntityId<'a>) -> I,
{
    let child_errors = namespace
        .namespaces()
        .flat_map(|child| recurse_namespaces(api, child, entity_id.child(child.name), f));

    child_errors
        .chain(f(api, namespace, entity_id.clone()))
        .collect_vec()
}

#[cfg(test)]
mod tests {
    use crate::model::{Builder, Model, ValidationError};
    use crate::test_util::executor::TestExecutor;

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
            use crate::model::builder::tests::merge::{
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
                            ..Default::default()
                        })
                    ]
                );
            }
        }

        mod merge_from_chunk {
            use std::path::PathBuf;

            use crate::model::builder::tests::merge::test_namespace;
            use crate::model::builder::tests::test_builder;
            use crate::model::{Builder, EntityId};
            use crate::test_util::executor::TestExecutor;
            use crate::{input, model};

            #[test]
            fn adds_chunk_metadata_with_current_namespace() {
                let mut builder = Builder::default();
                let file_path = Some(PathBuf::from("some/path"));
                builder.enter_namespace("blah");
                builder.merge_from_chunk(
                    test_namespace(1),
                    &input::Chunk {
                        data: "unused".to_string(),
                        relative_file_path: file_path.clone(),
                    },
                );
                assert_eq!(builder.metadata.chunks.len(), 1);
                let chunk_metadata = builder.metadata.chunks.get(0).unwrap();
                assert_eq!(chunk_metadata.root_namespace, EntityId::from(["blah"]));
                assert_eq!(chunk_metadata.relative_file_path, file_path);
            }

            #[test]
            fn applies_chunk_attr_to_all_entities_recursively() {
                let mut exe = TestExecutor::new("mod ns0 { mod ns1 {} struct dto {} }");
                let mut builder = test_builder(&mut exe);
                builder.enter_namespace("ns0");

                let mut exe = TestExecutor::new(
                    r#"
                    mod ns2 {
                        struct dto {}
                        fn rpc() {}
                    }
                "#,
                );
                let to_merge = exe.api();

                let file_path = PathBuf::from("some/path");
                builder.merge_from_chunk(
                    to_merge,
                    &input::Chunk {
                        data: "unused".to_string(),
                        relative_file_path: Some(file_path.clone()),
                    },
                );

                let api = builder.build().unwrap().api;
                // Existing shouldn't have attribute.
                assert!(!chunk_attr_contains_file_path(
                    &api.find_namespace(&["ns0"].into()).unwrap().attributes,
                    &file_path
                ));
                assert!(!chunk_attr_contains_file_path(
                    &api.find_namespace(&["ns0", "ns1"].into())
                        .unwrap()
                        .attributes,
                    &file_path
                ));
                assert!(!chunk_attr_contains_file_path(
                    &api.find_dto(&["ns0", "dto"].into()).unwrap().attributes,
                    &file_path
                ));
                // Merged should have correct attribute.
                assert!(chunk_attr_contains_file_path(
                    &api.find_namespace(&["ns0", "ns2"].into())
                        .unwrap()
                        .attributes,
                    &file_path
                ));
                assert!(chunk_attr_contains_file_path(
                    &api.find_dto(&["ns0", "ns2", "dto"].into())
                        .unwrap()
                        .attributes,
                    &file_path
                ));
                assert!(chunk_attr_contains_file_path(
                    &api.find_rpc(&["ns0", "ns2", "rpc"].into())
                        .unwrap()
                        .attributes,
                    &file_path
                ));
            }

            fn chunk_attr_contains_file_path(
                attr: &model::Attributes,
                file_path: &PathBuf,
            ) -> bool {
                attr.chunk
                    .as_ref()
                    .map(|attr| attr.relative_file_paths.contains(&file_path))
                    .unwrap_or(false)
            }
        }

        mod has_current_namespace {
            use crate::model::builder::tests::merge::{
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

            fn test_builder<'a>() -> Builder<'a> {
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
                ..Default::default()
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
                ..Default::default()
            }
        }
    }

    mod build {
        mod dedupe_namespaces {
            use crate::model::builder::tests::build_from_input;
            use crate::test_util::executor::TestExecutor;

            #[test]
            fn within_root() {
                let mut exe = TestExecutor::new(
                    r#"
                    mod ns2 {}
                    mod ns1 {}
                    mod ns1 {}
                    mod ns3 {}
                    mod ns2 {}
                    mod ns1 {}
                "#,
                );
                let model = build_from_input(&mut exe).unwrap();

                assert_eq!(model.api.namespaces().count(), 3);
                assert!(model.api.namespace("ns1").is_some());
                assert!(model.api.namespace("ns2").is_some());
                assert!(model.api.namespace("ns3").is_some());
            }

            #[test]
            fn within_sub_namespace() {
                let mut exe = TestExecutor::new(
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
                let model = build_from_input(&mut exe).unwrap();

                assert_eq!(model.api.namespaces().count(), 1);
                assert!(model.api.namespace("ns").is_some());

                let nested = model.api.namespace("ns").unwrap();
                assert_eq!(nested.namespaces().count(), 3);
                assert!(nested.namespace("ns1").is_some());
                assert!(nested.namespace("ns2").is_some());
                assert!(nested.namespace("ns3").is_some());
            }

            #[test]
            fn across_sub_namespaces_with_same_name() {
                let mut exe = TestExecutor::new(
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
                let model = build_from_input(&mut exe).unwrap();

                assert_eq!(model.api.namespaces().count(), 1);
                let nested = model.api.namespace("ns").unwrap();
                assert_eq!(nested.namespaces().count(), 3);
                assert!(nested.namespace("ns1").is_some());
                assert!(nested.namespace("ns2").is_some());
                assert!(nested.namespace("ns3").is_some());
            }
        }

        mod validate_duplicates {
            use crate::model::builder::tests::{assert_contains_error, build_from_input};
            use crate::model::builder::ValidationError;
            use crate::test_util::executor::TestExecutor;

            #[test]
            fn dtos() {
                let mut exe = TestExecutor::new(
                    r#"
                    mod ns {
                        struct dto {}
                        struct dto {}
                    }
                "#,
                );
                let result = build_from_input(&mut exe);
                assert_contains_error(&result, ValidationError::DuplicateDto(["ns", "dto"].into()));
            }

            #[test]
            fn rpcs() {
                let mut exe = TestExecutor::new(
                    r#"
                    mod ns {
                        fn rpc() {}
                        fn rpc() {}
                    }
                "#,
                );
                let result = build_from_input(&mut exe);
                assert_contains_error(&result, ValidationError::DuplicateRpc(["ns", "rpc"].into()));
            }

            #[test]
            fn rpc_dto_with_same_name_ok() {
                let mut exe = TestExecutor::new(
                    r#"
                    mod ns {
                        fn thing() {}
                        struct thing {}
                    }
                "#,
                );
                let result = build_from_input(&mut exe);
                assert!(result.is_ok());
            }
        }

        mod validate_dto {
            use crate::model::builder::tests::{
                assert_contains_error, build_from_input, test_builder,
            };
            use crate::model::builder::ValidationError;
            use crate::model::EntityId;
            use crate::test_util::executor::TestExecutor;

            #[test]
            fn name_empty() {
                let mut exe = TestExecutor::new(
                    r#"
                    mod ns {
                        struct dto0 {}
                        struct dto1 {}
                        struct dto2 {}
                    }
                "#,
                );
                let mut builder = test_builder(&mut exe);
                builder
                    .api
                    .find_dto_mut(&["ns", "dto2"].into())
                    .unwrap()
                    .name = "";

                let result = builder.build();
                let expected_entity_id = EntityId::from(["ns"]);
                let expected_index = 2;
                assert_contains_error(
                    &result,
                    ValidationError::InvalidDtoName(expected_entity_id, expected_index),
                );
            }

            #[test]
            fn field_name_empty() {
                let mut exe = TestExecutor::new(
                    r#"
                    mod ns {
                        struct dto {
                            field0: bool,
                            field1: bool,
                            field2: bool,
                        }
                    }"#,
                );
                let mut builder = test_builder(&mut exe);
                builder
                    .api
                    .find_dto_mut(&["ns", "dto"].into())
                    .unwrap()
                    .field_mut("field1")
                    .unwrap()
                    .name = "";

                let result = builder.build();
                let expected_entity_id = EntityId::from(["ns", "dto"]);
                let expected_index = 1;
                assert_contains_error(
                    &result,
                    ValidationError::InvalidFieldName(expected_entity_id, expected_index),
                );
            }

            #[test]
            fn field_type_invalid_linkage() {
                let mut exe = TestExecutor::new(
                    r#"
                    struct dto {
                        field0: bool,
                        field1: ns::dto
                    }
                    mod ns {
                        struct definitely_not_dto {}
                    }"#,
                );
                let result = build_from_input(&mut exe);
                let expected_index = 1;
                assert_contains_error(
                    &result,
                    ValidationError::InvalidFieldType(
                        ["dto"].into(),
                        "field1",
                        expected_index,
                        ["ns", "dto"].into(),
                    ),
                );
            }
        }

        mod validate_rpc {
            use crate::model::builder::tests::{
                assert_contains_error, build_from_input, test_builder,
            };
            use crate::model::builder::ValidationError;
            use crate::model::EntityId;
            use crate::test_util::executor::TestExecutor;

            #[test]
            fn name_empty() {
                let mut exe = TestExecutor::new(
                    r#"
                    mod ns {
                        fn rpc0() {}
                        fn rpc1() {}
                        fn rpc2() {}
                    }
                "#,
                );
                let mut builder = test_builder(&mut exe);
                builder
                    .api
                    .find_rpc_mut(&["ns", "rpc2"].into())
                    .unwrap()
                    .name = "";

                let result = builder.build();
                let expected_entity_id = EntityId::from(["ns"]);
                let expected_index = 2;
                assert_contains_error(
                    &result,
                    ValidationError::InvalidRpcName(expected_entity_id, expected_index),
                );
            }

            #[test]
            fn param_name_empty() {
                let mut exe = TestExecutor::new(
                    r#"
                    mod ns {
                        fn rpc(param0: bool, param1: bool, param2: bool) {}
                    }"#,
                );
                let mut builder = test_builder(&mut exe);
                builder
                    .api
                    .find_rpc_mut(&["ns", "rpc"].into())
                    .unwrap()
                    .param_mut("param1")
                    .unwrap()
                    .name = "";

                let result = builder.build();
                let expected_entity_id = EntityId::from(["ns", "rpc"]);
                let expected_index = 1;
                assert_contains_error(
                    &result,
                    ValidationError::InvalidFieldName(expected_entity_id, expected_index),
                );
            }

            #[test]
            fn param_type_invalid_linkage() {
                let mut exe = TestExecutor::new(
                    r#"
                    fn rpc(param0: bool, param1: ns::dto) {}
                    mod ns {
                        struct definitely_not_dto {}
                    }"#,
                );
                let result = build_from_input(&mut exe);
                let expected_index = 1;
                assert_contains_error(
                    &result,
                    ValidationError::InvalidFieldType(
                        ["rpc"].into(),
                        "param1",
                        expected_index,
                        ["ns", "dto"].into(),
                    ),
                );
            }

            #[test]
            fn return_type_invalid_linkage() {
                let mut exe = TestExecutor::new(
                    r#"
                    fn rpc() -> ns::dto {}
                    mod ns {
                        struct definitely_not_dto {}
                    }"#,
                );
                let result = build_from_input(&mut exe);
                assert_contains_error(
                    &result,
                    ValidationError::InvalidRpcReturnType(["rpc"].into(), ["ns", "dto"].into()),
                );
            }
        }

        mod validate_namespace {
            use crate::model::builder::tests::{assert_contains_error, build_from_input};
            use crate::model::builder::ValidationError;
            use crate::model::{Builder, UNDEFINED_NAMESPACE};
            use crate::test_util::executor::TestExecutor;

            #[test]
            fn root_namespace_undefined_allowed() {
                let builder = Builder::default();
                assert!(builder.build().is_ok());
            }

            #[test]
            fn name_within_root_not_undefined() {
                let mut exe = TestExecutor::new("mod zzzz {}".replace("zzzz", UNDEFINED_NAMESPACE));
                assert_contains_error(
                    &build_from_input(&mut exe),
                    ValidationError::InvalidNamespaceName([UNDEFINED_NAMESPACE].into()),
                );
            }

            #[test]
            fn name_below_root_not_undefined() {
                let mut exe = TestExecutor::new(
                    r#"
                    mod ns {
                        mod zzzz {}
                    }"#
                    .replace("zzzz", UNDEFINED_NAMESPACE),
                );
                assert_contains_error(
                    &build_from_input(&mut exe),
                    ValidationError::InvalidNamespaceName(["ns", UNDEFINED_NAMESPACE].into()),
                );
            }
        }
    }

    fn build_from_input(exe: &mut TestExecutor) -> Result<Model, Vec<ValidationError<'_>>> {
        test_builder(exe).build()
    }

    fn test_builder(exe: &mut TestExecutor) -> Builder {
        Builder {
            api: exe.api(),
            ..Default::default()
        }
    }

    fn assert_contains_error(
        build_result: &Result<Model, Vec<ValidationError<'_>>>,
        error: ValidationError,
    ) {
        let errors = build_result
            .as_ref()
            .map(|_| "...but it passed!")
            .expect_err("expected Builder::build to fail");
        assert!(
            errors.contains(&error),
            "missing: {:?}\nactual: {:?}",
            error,
            errors
        );
    }
}
