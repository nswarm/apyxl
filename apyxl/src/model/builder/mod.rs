use std::borrow::Cow;

use anyhow::Result;
use itertools::Itertools;
use log::{debug, error};

pub use config::*;

use crate::model::api::validate;
use crate::model::validate::ValidationResult;
use crate::model::{
    chunk, Api, Chunk, EntityId, EntityType, Metadata, Model, Namespace, ValidationError,
    UNDEFINED_NAMESPACE,
};
use crate::{generator, output, Generator};

mod config;

/// Helper struct made for parsing [Api]s spread across multiple [Chunk]s. Tracks [Metadata]
/// associated with entities in the [Api]s.
///
/// After all desired [Api]s are merged, the [Builder] can be finalized via [Builder::build] which will
/// perform validation across the entire [Api] and return the final [Model].
pub struct Builder<'a> {
    config: Config,
    api: Api<'a>,
    namespace_stack: Vec<String>,
    metadata: Metadata,
}

impl Default for Builder<'_> {
    fn default() -> Self {
        Self {
            api: Api {
                name: Cow::Borrowed(UNDEFINED_NAMESPACE),
                ..Default::default()
            },
            config: Default::default(),
            namespace_stack: Default::default(),
            metadata: Default::default(),
        }
    }
}

impl<'a> Builder<'a> {
    pub fn with_config(config: Config) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

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
    pub fn merge_from_chunk(&mut self, mut namespace: Namespace<'a>, chunk: &Chunk) {
        if let Some(relative_file_path) = &chunk.relative_file_path {
            let root_namespace = self.current_namespace_id();
            self.metadata_mut().chunks.push(chunk::Metadata {
                root_namespace,
                chunk: chunk.clone(),
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
    pub fn enter_namespace<S: ToString>(&mut self, name: S) {
        let name = name.to_string();
        if self.current_namespace().namespace(&name).is_none() {
            self.current_namespace_mut().add_namespace(Namespace {
                name: Cow::Owned(name.clone()),
                ..Default::default()
            });
        }
        self.namespace_stack.push(name);
        debug!("entered namespace: {:?}", self.namespace_stack);
    }

    /// Remove the most recently-added namespace from the stack.
    pub fn exit_namespace(&mut self) {
        debug!("exited namespace: {:?}", self.namespace_stack);
        self.namespace_stack.pop();
    }

    /// Clear the entire namespace stack.
    pub fn clear_namespace(&mut self) {
        debug!("clear namespace: ({:?})", self.namespace_stack);
        self.namespace_stack.clear()
    }

    /// Finalize and validate the model.
    pub fn build(mut self) -> Result<Model<'a>, Vec<ValidationError>> {
        dedupe_namespace_children(&mut self.api);

        self.pre_validation_print();

        let (oks, mut errs): (Vec<_>, Vec<_>) = [
            validate::recurse_api(&self.api, validate::namespace_names),
            validate::recurse_api(&self.api, validate::dto_names),
            validate::recurse_api(&self.api, validate::dto_field_names),
            validate::recurse_api(&self.api, validate::dto_field_names_no_duplicates),
            validate::recurse_api(&self.api, validate::dto_field_types),
            validate::recurse_api(&self.api, validate::rpc_names),
            validate::recurse_api(&self.api, validate::rpc_param_names),
            validate::recurse_api(&self.api, validate::rpc_param_names_no_duplicates),
            validate::recurse_api(&self.api, validate::rpc_param_types),
            validate::recurse_api(&self.api, validate::rpc_return_types),
            validate::recurse_api(&self.api, validate::ty_alias_target_type),
            validate::recurse_api(&self.api, validate::enum_names),
            validate::recurse_api(&self.api, validate::enum_value_names),
            validate::recurse_api(&self.api, validate::ty_alias_names),
            validate::recurse_api(&self.api, validate::no_duplicate_dto_enum_alias),
            validate::recurse_api(&self.api, validate::no_duplicate_rpcs),
            validate::recurse_api(&self.api, validate::no_duplicate_enum_value_names),
        ]
        .into_iter()
        .flatten()
        .partition(|x| x.is_ok());

        errs.append(&mut devirtualize_namespaces(
            &mut self.api,
            &EntityId::default(),
        ));

        if !errs.is_empty() {
            return Err(errs.into_iter().map(Result::unwrap_err).collect_vec());
        }

        for mutation in oks.into_iter().filter_map(Result::unwrap) {
            mutation.execute(&mut self.api).unwrap();
        }

        Ok(Model::new(self.api, self.metadata))
    }

    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    pub fn metadata_mut(&mut self) -> &mut Metadata {
        &mut self.metadata
    }

    pub fn current_namespace_id(&self) -> EntityId {
        EntityId::new_unqualified_vec(self.namespace_stack.iter())
    }

    pub fn current_namespace(&self) -> &Namespace<'a> {
        self.api.find_namespace(&self.current_namespace_id())
            .expect("enter_namespace must always create the namespace if it does not exist, which will guarantee this never fails")
    }

    pub fn current_namespace_mut(&mut self) -> &mut Namespace<'a> {
        let entity_id = self.current_namespace_id();
        self.api.find_namespace_mut(&entity_id)
            .expect("enter_namespace must always create the namespace if it does not exist, which will guarantee this never fails")
    }

    #[cfg(test)]
    pub fn into_api(self) -> Api<'a> {
        self.api
    }

    fn pre_validation_print(&self) {
        match self.config.debug_pre_validate_print {
            PreValidatePrint::None => {}
            PreValidatePrint::Rust => pretty_print_api(&self.api),
            PreValidatePrint::Debug => println!("pre-validation API: {:#?}", self.api),
        }
    }
}

fn dedupe_namespace_children(namespace: &mut Namespace) {
    namespace
        .take_namespaces()
        .into_iter()
        .sorted_unstable_by_key(|ns| ns.name.to_string())
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

/// Moves any namespaces marked as 'virtual' into DTOs with the same name.
fn devirtualize_namespaces(
    namespace: &mut Namespace,
    namespace_id: &EntityId,
) -> Vec<ValidationResult> {
    namespace
        .take_namespaces_filtered(|namespace| namespace.is_virtual)
        .into_iter()
        .flat_map(|mut virtual_namespace| {
            let virtual_ns_id = namespace_id
                .child(EntityType::Namespace, &virtual_namespace.name)
                .unwrap();
            let mut results = devirtualize_namespaces(&mut virtual_namespace, &virtual_ns_id);
            match namespace.dto_mut(&virtual_namespace.name) {
                None => {
                    results.push(Err(ValidationError::VirtualNamespaceMissingOwner(
                        virtual_ns_id.clone(),
                    )));
                }
                Some(dto) => {
                    dto.namespace = Some(virtual_namespace);
                }
            }
            results
        })
        .collect_vec()
}

fn pretty_print_api(api: &Api) {
    let model = Model::new(api.clone(), Metadata::default());
    let mut output = output::Buffer::default();
    match generator::Rust::default().generate(model.view(), &mut output) {
        Ok(_) => {
            println!("pre-validation API:\n{}", output.to_string());
        }
        Err(err) => error!(
            "error when generating pre-validation API for printing: {}",
            err
        ),
    }
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
        use std::borrow::Cow;

        use crate::model::{Dto, Namespace, NamespaceChild};

        mod no_current_namespace {
            use std::borrow::Cow;

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
                            name: Cow::Borrowed(NS_NAMES[1]),
                            children: vec![test_child_dto(2)],
                            ..Default::default()
                        })
                    ]
                );
            }
        }

        mod merge_from_chunk {
            use std::path::PathBuf;

            use crate::model;
            use crate::model::builder::tests::merge::test_namespace;
            use crate::model::builder::tests::test_builder;
            use crate::model::{Builder, Chunk, EntityId};
            use crate::test_util::executor::TestExecutor;

            #[test]
            fn adds_chunk_metadata_with_current_namespace() {
                let mut builder = Builder::default();
                let file_path = Some(PathBuf::from("some/path"));
                builder.enter_namespace("blah");
                builder.merge_from_chunk(
                    test_namespace(1),
                    &Chunk::with_relative_file_path(file_path.clone().unwrap()),
                );
                assert_eq!(builder.metadata.chunks.len(), 1);
                let chunk_metadata = builder.metadata.chunks.get(0).unwrap();
                assert_eq!(
                    chunk_metadata.root_namespace,
                    EntityId::new_unqualified("blah")
                );
                assert_eq!(chunk_metadata.chunk.relative_file_path, file_path);
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
                builder
                    .merge_from_chunk(to_merge, &Chunk::with_relative_file_path(file_path.clone()));

                let api = builder.build().unwrap().api;
                // Existing shouldn't have attribute.
                assert!(!chunk_attr_contains_file_path(
                    &api.find_namespace(&EntityId::new_unqualified("ns0"))
                        .unwrap()
                        .attributes,
                    &file_path
                ));
                assert!(!chunk_attr_contains_file_path(
                    &api.find_namespace(&EntityId::new_unqualified("ns0.ns1"))
                        .unwrap()
                        .attributes,
                    &file_path
                ));
                assert!(!chunk_attr_contains_file_path(
                    &api.find_dto(&EntityId::new_unqualified("ns0.dto"))
                        .unwrap()
                        .attributes,
                    &file_path
                ));
                // Merged should have correct attribute.
                assert!(chunk_attr_contains_file_path(
                    &api.find_namespace(&EntityId::new_unqualified("ns0.ns2"))
                        .unwrap()
                        .attributes,
                    &file_path
                ));
                assert!(chunk_attr_contains_file_path(
                    &api.find_dto(&EntityId::new_unqualified("ns0.ns2.dto"))
                        .unwrap()
                        .attributes,
                    &file_path
                ));
                assert!(chunk_attr_contains_file_path(
                    &api.find_rpc(&EntityId::new_unqualified("ns0.ns2.rpc"))
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
                name: Cow::Borrowed(name),
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

        mod devirtualize_namespaces {
            use crate::model::builder::tests::build_from_input;
            use crate::model::{EntityId, ValidationError};
            use crate::test_util::executor::TestExecutor;

            #[test]
            fn moves_into_dto_namespace() {
                let mut exe = TestExecutor::new(
                    r#"
                    struct dto {}
                    impl dto {
                        type NestedAlias = u32;
                        fn nested_rpc() {}
                    }
                "#,
                );
                let model = build_from_input(&mut exe).unwrap();

                let dto = model.api.dto("dto").unwrap();
                let dto_namespace = dto.namespace.as_ref().unwrap();
                assert!(dto_namespace.rpc("nested_rpc").is_some());
                assert!(dto_namespace.ty_alias("NestedAlias").is_some());
            }

            #[test]
            fn errors_if_no_owning_dto() {
                let mut exe = TestExecutor::new(
                    r#"
                    struct dto {}
                    impl wrong_name {
                        fn nested_rpc() {}
                    }
                "#,
                );
                let errors = build_from_input(&mut exe).unwrap_err();
                assert_eq!(errors.len(), 1);
                assert_eq!(
                    errors[0],
                    ValidationError::VirtualNamespaceMissingOwner(
                        EntityId::try_from("wrong_name").unwrap()
                    )
                );
            }
        }

        mod validate_duplicates {
            use crate::model::builder::tests::{assert_contains_error, build_from_input};
            use crate::model::builder::ValidationError;
            use crate::model::EntityId;
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
                assert_contains_error(
                    &result,
                    ValidationError::DuplicateDtoOrEnumOrAlias(EntityId::new_unqualified("ns.dto")),
                );
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
                assert_contains_error(
                    &result,
                    ValidationError::DuplicateRpc(EntityId::try_from("ns.r:rpc").unwrap()),
                );
            }

            #[test]
            fn enums() {
                let mut exe = TestExecutor::new(
                    r#"
                    mod ns {
                        enum en {}
                        enum en {}
                    }
                "#,
                );
                let result = build_from_input(&mut exe);
                assert_contains_error(
                    &result,
                    ValidationError::DuplicateDtoOrEnumOrAlias(EntityId::new_unqualified("ns.en")),
                );
            }

            #[test]
            fn enum_dto() {
                let mut exe = TestExecutor::new(
                    r#"
                    mod ns {
                        struct asdf {}
                        enum asdf {}
                    }
                "#,
                );
                let result = build_from_input(&mut exe);
                assert_contains_error(
                    &result,
                    ValidationError::DuplicateDtoOrEnumOrAlias(EntityId::new_unqualified(
                        "ns.asdf",
                    )),
                );
            }

            #[test]
            fn enum_alias() {
                let mut exe = TestExecutor::new(
                    r#"
                    mod ns {
                        enum asdf {}
                        type asdf = u32;
                    }
                "#,
                );
                let result = build_from_input(&mut exe);
                assert_contains_error(
                    &result,
                    ValidationError::DuplicateDtoOrEnumOrAlias(EntityId::new_unqualified(
                        "ns.asdf",
                    )),
                );
            }

            #[test]
            fn dto_alias() {
                let mut exe = TestExecutor::new(
                    r#"
                    mod ns {
                        struct asdf {}
                        type asdf = u32;
                    }
                "#,
                );
                let result = build_from_input(&mut exe);
                assert_contains_error(
                    &result,
                    ValidationError::DuplicateDtoOrEnumOrAlias(EntityId::new_unqualified(
                        "ns.asdf",
                    )),
                );
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
                    .find_dto_mut(&EntityId::new_unqualified("ns.dto2"))
                    .unwrap()
                    .name = "";

                let result = builder.build();
                let expected_entity_id = EntityId::try_from("ns").unwrap();
                let expected_index = 2;
                assert_contains_error(
                    &result,
                    ValidationError::InvalidDtoName(expected_entity_id.to_owned(), expected_index),
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
                    .find_dto_mut(&EntityId::new_unqualified("ns.dto"))
                    .unwrap()
                    .field_mut("field1")
                    .unwrap()
                    .name = "";

                let result = builder.build();
                let expected_entity_id = EntityId::try_from("ns.d:dto").unwrap();
                let expected_index = 1;
                assert_contains_error(
                    &result,
                    ValidationError::InvalidFieldName(
                        expected_entity_id.to_owned(),
                        expected_index,
                    ),
                );
            }

            #[test]
            fn field_name_duplicates() {
                let mut exe = TestExecutor::new(
                    r#"
                    mod ns {
                        struct dto {
                            field: bool,
                            field: bool,
                        }
                    }"#,
                );
                let builder = test_builder(&mut exe);
                let result = builder.build();
                let expected_entity_id = EntityId::try_from("ns.d:dto").unwrap();
                assert_contains_error(
                    &result,
                    ValidationError::DuplicateFieldName(
                        expected_entity_id.to_owned(),
                        "field".to_string(),
                    ),
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
                        EntityId::try_from("d:dto").unwrap(),
                        "field1".to_string(),
                        expected_index,
                        EntityId::new_unqualified("ns.dto"),
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
                    .find_rpc_mut(&EntityId::new_unqualified("ns.rpc2"))
                    .unwrap()
                    .name = "";

                let result = builder.build();
                let expected_entity_id = EntityId::try_from("ns").unwrap();
                let expected_index = 2;
                assert_contains_error(
                    &result,
                    ValidationError::InvalidRpcName(expected_entity_id.to_owned(), expected_index),
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
                    .find_rpc_mut(&EntityId::new_unqualified("ns.rpc"))
                    .unwrap()
                    .param_mut("param1")
                    .unwrap()
                    .name = "";

                let result = builder.build();
                let expected_entity_id = EntityId::try_from("ns.r:rpc").unwrap();
                let expected_index = 1;
                assert_contains_error(
                    &result,
                    ValidationError::InvalidFieldName(
                        expected_entity_id.to_owned(),
                        expected_index,
                    ),
                );
            }

            #[test]
            fn param_name_duplicates() {
                let mut exe = TestExecutor::new(
                    r#"
                    mod ns {
                        fn rpc(param: bool, param: bool) {}
                    }"#,
                );
                let builder = test_builder(&mut exe);
                let result = builder.build();
                let expected_entity_id = EntityId::try_from("ns.r:rpc").unwrap();
                assert_contains_error(
                    &result,
                    ValidationError::DuplicateFieldName(
                        expected_entity_id.to_owned(),
                        "param".to_string(),
                    ),
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
                        EntityId::try_from("r:rpc").unwrap(),
                        "param1".to_string(),
                        expected_index,
                        EntityId::new_unqualified("ns.dto"),
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
                    ValidationError::InvalidRpcReturnType(
                        EntityId::try_from("r:rpc").unwrap(),
                        EntityId::new_unqualified("ns.dto"),
                    ),
                );
            }
        }

        mod validate_enum {
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
                        enum en0 {}
                        enum en1 {}
                        enum en2 {}
                    }
                "#,
                );
                let mut builder = test_builder(&mut exe);
                builder
                    .api
                    .find_enum_mut(&EntityId::new_unqualified("ns.en2"))
                    .unwrap()
                    .name = "";

                let result = builder.build();
                let expected_entity_id = EntityId::try_from("ns").unwrap();
                let expected_index = 2;
                assert_contains_error(
                    &result,
                    ValidationError::InvalidEnumName(expected_entity_id.to_owned(), expected_index),
                );
            }

            #[test]
            fn value_name_empty() {
                let mut exe = TestExecutor::new(
                    r#"
                    mod ns {
                        enum en {
                            value0 = 0,
                            value1 = 1,
                            value2 = 2,
                        }
                    }"#,
                );
                let mut builder = test_builder(&mut exe);
                builder
                    .api
                    .find_enum_mut(&EntityId::new_unqualified("ns.en"))
                    .unwrap()
                    .value_mut("value1")
                    .unwrap()
                    .name = "";

                let result = builder.build();
                let expected_entity_id = EntityId::try_from("ns.e:en").unwrap();
                let expected_index = 1;
                assert_contains_error(
                    &result,
                    ValidationError::InvalidEnumValueName(
                        expected_entity_id.to_owned(),
                        expected_index,
                    ),
                );
            }

            #[test]
            fn duplicate_value_names() {
                let mut exe = TestExecutor::new(
                    r#"
                    mod ns {
                        enum en {
                            val = 0,
                            val = 1,
                            val = 2,
                        }
                    }
                "#,
                );
                let result = build_from_input(&mut exe);
                assert_contains_error(
                    &result,
                    ValidationError::DuplicateEnumValue(
                        EntityId::try_from("ns.e:en").unwrap(),
                        "val".to_string(),
                    ),
                );
            }
        }

        mod validate_ty_alias {
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
                        type alias0 = u32;
                        type alias1 = u32;
                        type alias2 = u32;
                    }
                "#,
                );
                let mut builder = test_builder(&mut exe);
                builder
                    .api
                    .find_ty_alias_mut(&EntityId::new_unqualified("ns.alias2"))
                    .unwrap()
                    .name = "";

                let result = builder.build();
                let expected_entity_id = EntityId::try_from("ns").unwrap();
                let expected_index = 2;
                assert_contains_error(
                    &result,
                    ValidationError::InvalidTypeAliasName(
                        expected_entity_id.to_owned(),
                        expected_index,
                    ),
                );
            }

            #[test]
            fn target_ty_invalid_linkage() {
                let mut exe = TestExecutor::new(
                    r#"
                    type alias = ns::dto;
                    mod ns {
                        struct definitely_not_dto {}
                    }"#,
                );
                let result = build_from_input(&mut exe);
                assert_contains_error(
                    &result,
                    ValidationError::InvalidTypeAliasTargetType(
                        EntityId::try_from("a:alias").unwrap(),
                        EntityId::new_unqualified("ns.dto"),
                    ),
                );
            }
        }

        mod validate_namespace {
            use crate::model::builder::tests::{assert_contains_error, build_from_input};
            use crate::model::builder::ValidationError;
            use crate::model::{Builder, EntityId, UNDEFINED_NAMESPACE};
            use crate::test_util::executor::TestExecutor;

            #[test]
            fn root_namespace_undefined_allowed() {
                let builder = Builder::default();
                assert!(builder.build().is_ok());
            }

            #[test]
            fn name_within_root_not_undefined() {
                let mut exe = TestExecutor::new("mod asdf {}".replace("asdf", UNDEFINED_NAMESPACE));
                assert_contains_error(
                    &build_from_input(&mut exe),
                    ValidationError::InvalidNamespaceName(
                        EntityId::new_unqualified(UNDEFINED_NAMESPACE).to_qualified_namespaces(),
                    ),
                );
            }

            #[test]
            fn name_below_root_not_undefined() {
                let mut exe = TestExecutor::new(
                    r#"
                    mod ns {
                        mod asdf {}
                    }"#
                    .replace("asdf", UNDEFINED_NAMESPACE),
                );
                assert_contains_error(
                    &build_from_input(&mut exe),
                    ValidationError::InvalidNamespaceName(
                        EntityId::new_unqualified_vec(["ns", UNDEFINED_NAMESPACE].iter())
                            .to_qualified_namespaces(),
                    ),
                );
            }
        }

        mod metadata {
            use std::path::PathBuf;

            use anyhow::Result;

            use crate::model::{chunk, Builder, EntityId, ValidationError};

            #[test]
            fn passed_through() -> Result<(), Vec<ValidationError>> {
                let mut builder = Builder::default();
                let chunk_metadata = chunk::Metadata {
                    root_namespace: EntityId::new_unqualified("hi"),
                    chunk: chunk::Chunk::with_relative_file_path(PathBuf::from("hi")),
                };
                builder.metadata.chunks.push(chunk_metadata.clone());
                let model = builder.build()?;
                let actual_chunk_metadata = model.metadata.chunks.get(0).unwrap();
                assert_eq!(
                    actual_chunk_metadata.root_namespace,
                    chunk_metadata.root_namespace
                );
                assert_eq!(actual_chunk_metadata.chunk, chunk_metadata.chunk);
                Ok(())
            }
        }

        mod qualifies_types {
            use crate::model::entity::FindEntity;
            use crate::model::{Api, Entity, EntityId};
            use crate::test_util::executor::TestExecutor;

            #[test]
            fn dto_field_types() {
                let mut exe = TestExecutor::new(
                    r#"
                    mod ns0 {
                        struct dep0 {}
                        mod ns1 {
                            enum dep1 {}
                        }
                    }
                    mod ns2 {
                        struct dto {
                            zero: ns0::dep0,
                            one: ns0::ns1::dep1,
                        }
                    }
                "#,
                );
                let model = exe.build();

                assert_qualified_ty(&model.api, "ns2.d:dto.f:zero.ty", "ns0.dto:dep0");
                assert_qualified_ty(&model.api, "ns2.d:dto.f:one.ty", "ns0.ns1.enum:dep1");
            }

            #[test]
            fn rpc_param_types() {
                let mut exe = TestExecutor::new(
                    r#"
                    mod ns0 {
                        struct dep0 {}
                        mod ns1 {
                            enum dep1 {}
                        }
                    }
                    mod ns2 {
                        fn rpc(
                            zero: ns0::dep0,
                            one: ns0::ns1::dep1,
                        ) {}
                    }
                "#,
                );
                let model = exe.build();

                assert_qualified_ty(&model.api, "ns2.r:rpc.f:zero.ty", "ns0.dto:dep0");
                assert_qualified_ty(&model.api, "ns2.r:rpc.f:one.ty", "ns0.ns1.enum:dep1");
            }

            #[test]
            fn rpc_return_type() {
                let mut exe = TestExecutor::new(
                    r#"
                    mod ns0 {
                        mod ns1 {
                            enum dep {}
                        }
                    }
                    mod ns2 {
                        fn rpc() -> ns0::ns1::dep {}
                    }
                "#,
                );
                let model = exe.build();

                assert_qualified_ty(&model.api, "ns2.r:rpc.return_ty", "ns0.ns1.enum:dep");
            }

            #[test]
            fn ty_alias_target_type() {
                let mut exe = TestExecutor::new(
                    r#"
                    mod ns0 {
                        mod ns1 {
                            enum dep {}
                        }
                    }
                    mod ns2 {
                        type alias = ns0::ns1::dep;
                    }
                "#,
                );
                let model = exe.build();

                assert_qualified_ty(&model.api, "ns2.a:alias.target_ty", "ns0.ns1.enum:dep");
            }

            fn assert_qualified_ty(api: &Api, ty_id: &str, expected_target_id: &str) {
                let ty_id = EntityId::try_from(ty_id).unwrap();
                let ty_entity = api.find_entity(ty_id).unwrap();
                if let Entity::Type(ty) = ty_entity {
                    let target = ty.api().unwrap();
                    assert!(target.is_qualified());
                    assert_eq!(target.to_string(), expected_target_id);
                } else {
                    panic!("found wrong type");
                }
            }
        }
    }

    fn build_from_input(exe: &mut TestExecutor) -> Result<Model, Vec<ValidationError>> {
        test_builder(exe).build()
    }

    fn test_builder(exe: &mut TestExecutor) -> Builder {
        Builder {
            api: exe.api(),
            ..Default::default()
        }
    }

    fn assert_contains_error(
        build_result: &Result<Model, Vec<ValidationError>>,
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
