use anyhow::Result;
use log::{info, log};
use thiserror::Error;

use crate::model::{Api, Namespace, TypeRef, UNDEFINED_NAMESPACE};

/// Helper struct for parsing [Namespace]s spread across multiple chunks.
/// After all desired [Namespace]s are merged, the [Builder] can be finalized via [Builder::build] which will
/// perform validation across the entire [Api].
pub struct Builder<'a> {
    api: Api<'a>,
    namespace_stack: Vec<&'a str>,
}

#[derive(Error, Debug)]
pub enum ValidationError<'a> {
    #[error("Cannot add a namespace with the name {}", UNDEFINED_NAMESPACE)]
    InvalidNamespaceName,

    #[error("Found duplicate DTO definition at path {0:?}")]
    DuplicateDto(TypeRef<'a>),

    #[error("Found duplicate RPC definition at path {0:?}")]
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
        dedupe_namespaces(&mut self.api)?;
        validate_namespace_names(&mut self.api)?;
        validate_no_duplicates(&mut self.api)?;
        validate_type_refs(&mut self.api)?;
        Ok(self.api)
    }

    // /// Merge [Namespace] `other` into this [Namespace] by adding all of `other`'s children to to
    // /// this [Namespace]'s children. `other`'s name is ignored. The process will continue through
    // /// non-fatal errors in order to produce as much error information as possible.
    // ///
    // /// Duplicate [Namespace]s will be merged, recursively.
    // /// Duplicate children of other child types will produce a [ValidationError].
    // pub fn merge(&mut self, other: Namespace<'a>) -> Result<(), Vec<ValidationError>> {
    //     let mut errors = Vec::new();
    //     for child in other.children {
    //         match child {
    //             NamespaceChild::Namespace(child_namespace) => {
    //                 if let Some(existing) = self.namespace_mut(child_namespace.name) {
    //                     existing.merge(child_namespace)?;
    //                 } else {
    //                     self.add_namespace(child_namespace);
    //                 }
    //             }
    //             NamespaceChild::Dto(child_dto) => {
    //                 if let Some(existing) = self.dto(child_dto.name) {
    //                     errors.push(ValidationError::DuplicateDto(child_dto.))
    //                 } else {
    //                     self.add_namespace(child_namespace);
    //                 }
    //             }
    //             NamespaceChild::Rpc(_) => {}
    //         }
    //     }
    //     if namespace.name == UNDEFINED_NAMESPACE {
    //         return Err(MergeError);
    //     }
    //     if let Some(existing) = self.namespace_mut(namespace.name) {
    //         existing.merge(namespace)
    //     } else {
    //         Ok(())
    //     }
    //     self.children.append(&mut other.children);
    // }

    fn current_namespace(&self) -> &Namespace<'a> {
        self.api.find_namespace(&TypeRef::from(self.namespace_stack.as_slice()))
            .expect("enter_namespace must always create the namespace if it does not exist, which will guarantee this never fails")
    }

    fn current_namespace_mut(&mut self) -> &mut Namespace<'a> {
        self.api.find_namespace_mut(&TypeRef::from(self.namespace_stack.as_slice()))
            .expect("enter_namespace must always create the namespace if it does not exist, which will guarantee this never fails")
    }
}

fn dedupe_namespaces<'a>(namespace: &mut Namespace<'a>) -> Result<(), Vec<ValidationError<'a>>> {
    info!("deduping namespaces...");
    Ok(())
}

fn validate_namespace_names<'a>(
    namespace: &mut Namespace<'a>,
) -> Result<(), Vec<ValidationError<'a>>> {
    info!("validating namespace names...");
    Ok(())
}

fn validate_no_duplicates<'a>(
    namespace: &mut Namespace<'a>,
) -> Result<(), Vec<ValidationError<'a>>> {
    info!("validating no duplicate definitions...");
    Ok(())
}

fn validate_type_refs<'a>(namespace: &mut Namespace<'a>) -> Result<(), Vec<ValidationError<'a>>> {
    info!("validating type refs...");
    Ok(())
}

#[cfg(test)]
mod test {
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
            use crate::model::api::builder::test::merge::{
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
            use crate::model::api::builder::test::merge::{
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

        // validate typerefs not empty
        // validate typerefs all have real linkage
        // validate dtos/rpcs have required bits

        #[test]
        fn asdf() {
            todo!("nyi")
        }
    }
}
