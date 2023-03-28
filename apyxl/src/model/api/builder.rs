use anyhow::Result;

use crate::model::{Api, Namespace, Segment, TypeRef, UNDEFINED_NAMESPACE};

// todo description
pub struct Builder<'a> {
    api: Api<'a>,
    namespace_stack: Vec<&'a str>,
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
            // todo add_namespace?
            self.current_namespace_mut()
                .segments
                .push(Segment::Namespace(namespace))
        }
    }

    /// Add `namespace` to the current namespace stack of the Builder. Any [Api]s merged will be
    /// nested within the full namespace specified by the stack.
    pub fn enter_namespace(&mut self, name: &'a str) {
        // todo has_namespace?
        // if !self
        //     .current_namespace_mut()
        //     .namespaces()
        //     .any(|ns| ns.name == name)
        // {
        //     // todo add_namespace?
        //     // todo namespace new(name)?
        //     self.current_namespace_mut()
        //         .segments
        //         .push(Segment::Namespace(Namespace {
        //             name,
        //             segments: vec![],
        //         }))
        // }
        // self.namespace_stack.push(name);
    }

    /// Remove the most recently-added namespace from the stack.
    pub fn exit_namespace(&mut self) {
        self.namespace_stack.pop();
    }

    /// Finalize and validate the API.
    /// - De-dupes namespaces.
    /// - Errors for duplicate types.
    /// - Errors for TypeRefs with missing types not specified in list of primitives.
    // todo probably want a more complex error type here that can give info about >1 issue.
    pub fn finalize(self) -> Result<Api<'a>> {
        todo!("nyi")
    }

    // todo current_namespace (non-mut)
    // fn current_namespace_mut(&mut self) -> &mut Namespace<'a> {
    //     self.api.find_namespace_mut(&TypeRef::new(&self.namespace_stack))
    //         .expect("enter_namespace must always create the namespace if it does not exist, which will guarantee this never fails")
    // }

    fn current_namespace_mut(&mut self) -> &mut Namespace<'a> {
        // self.api.find_namespace_mut(&TypeRef::new(&self.namespace_stack))
        //     .expect("enter_namespace must always create the namespace if it does not exist, which will guarantee this never fails")
        todo!()
    }
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
        use crate::model::{Dto, Namespace, Segment};

        mod no_current_namespace {
            use crate::model::api::builder::test::merge::{
                test_dto_segment, test_named_namespace, test_namespace, test_namespace_segment,
                NS_NAMES,
            };
            use crate::model::{Builder, Namespace, Segment, UNDEFINED_NAMESPACE};

            #[test]
            fn name_is_empty() {
                let mut builder = Builder::default();
                builder.merge(test_named_namespace("", 1));
                assert_eq!(builder.api.name, UNDEFINED_NAMESPACE, "no change root name");
                assert_eq!(builder.api.segments, vec![test_dto_segment(1)]);
            }

            #[test]
            fn name_is_root() {
                let mut builder = Builder::default();
                builder.merge(test_named_namespace(UNDEFINED_NAMESPACE, 1));
                assert_eq!(builder.api.name, UNDEFINED_NAMESPACE, "no change root name");
                assert_eq!(builder.api.segments, vec![test_dto_segment(1)]);
            }

            #[test]
            fn name_is_new() {
                let mut builder = Builder::default();
                builder.api.segments.push(test_namespace_segment(1));
                builder.merge(test_namespace(2));
                assert_eq!(
                    builder.api.segments,
                    vec![test_namespace_segment(1), test_namespace_segment(2)]
                );
            }

            #[test]
            fn name_is_existing() {
                let mut builder = Builder::default();
                builder.api.segments.push(test_namespace_segment(1));
                builder.merge(test_named_namespace(NS_NAMES[1], 2));
                assert_eq!(
                    builder.api.segments,
                    vec![
                        // Duplicates preserved.
                        test_namespace_segment(1),
                        Segment::Namespace(Namespace {
                            name: NS_NAMES[1],
                            segments: vec![test_dto_segment(2)],
                        })
                    ]
                );
            }
        }

        mod has_current_namespace {
            use crate::model::api::builder::test::merge::{
                test_dto_segment, test_named_namespace, test_namespace, test_namespace_segment,
            };
            use crate::model::{Builder, Namespace, Segment, UNDEFINED_NAMESPACE};

            #[test]
            fn name_is_empty() {
                let mut builder = test_builder();
                builder.merge(test_named_namespace("", 1));
                assert_eq!(builder.api.name, UNDEFINED_NAMESPACE, "no change root name");

                let mut expected = current_namespace();
                expected.segments.push(test_dto_segment(1));
                assert_eq!(builder.api.segments, vec![Segment::Namespace(expected)]);
            }

            #[test]
            fn name_is_root() {
                let mut builder = test_builder();
                builder.merge(test_named_namespace(UNDEFINED_NAMESPACE, 1));
                assert_eq!(builder.api.name, UNDEFINED_NAMESPACE, "no change root name");

                let mut expected = current_namespace();
                expected.segments.push(test_dto_segment(1));
                assert_eq!(builder.api.segments, vec![Segment::Namespace(expected)]);
            }

            #[test]
            fn name_is_new() {
                let mut builder = test_builder();
                builder.merge(test_namespace(2));

                let mut expected = current_namespace();
                expected.segments.push(test_namespace_segment(2));
                assert_eq!(builder.api.segments, vec![Segment::Namespace(expected)]);
            }

            #[test]
            fn name_is_existing() {
                let mut builder = test_builder();
                if let Segment::Namespace(ns) = builder.api.segments.get_mut(0).unwrap() {
                    ns.segments.push(test_namespace_segment(2));
                }
                builder.merge(test_namespace(2));

                let mut expected = current_namespace();
                // Duplicates preserved.
                expected.segments.push(test_namespace_segment(2));
                expected.segments.push(test_namespace_segment(2));
                assert_eq!(builder.api.segments, vec![Segment::Namespace(expected)],);
            }

            const CURRENT_NAMESPACE: &str = "current";
            fn current_namespace() -> Namespace<'static> {
                test_named_namespace(CURRENT_NAMESPACE, 4)
            }

            fn test_builder() -> Builder<'static> {
                let mut builder = Builder::default();
                builder
                    .api
                    .segments
                    .push(Segment::Namespace(current_namespace()));
                builder.enter_namespace(CURRENT_NAMESPACE);
                builder
            }
        }

        mod finalize {
            #[test]
            fn asdf() {
                todo!("nyi")
            }
        }

        fn test_namespace_segment(i: usize) -> Segment<'static> {
            Segment::Namespace(test_namespace(i))
        }

        const NS_NAMES: &[&str] = &["ns0", "ns1", "ns2", "ns3", "ns4"];
        fn test_namespace(i: usize) -> Namespace<'static> {
            test_named_namespace(NS_NAMES[i], i)
        }

        fn test_named_namespace(name: &'static str, i: usize) -> Namespace<'static> {
            Namespace {
                name,
                segments: vec![test_dto_segment(i)],
            }
        }

        fn test_dto_segment(i: usize) -> Segment<'static> {
            Segment::Dto(test_dto(i))
        }

        const DTO_NAMES: &[&str] = &["DtoName0", "DtoName1", "DtoName2", "DtoName3", "DtoName4"];
        fn test_dto(i: usize) -> Dto<'static> {
            Dto {
                name: DTO_NAMES[i],
                fields: vec![],
            }
        }
    }
}
