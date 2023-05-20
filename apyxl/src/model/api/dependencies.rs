use crate::model::{Api, EntityId, Namespace, Type};
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;

pub type DependencyGraph = DiGraph<EntityId, ()>;

/// Tracks all dependencies across the API. Each [NamespaceChild] type is a node in the graph,
/// and each reference between namespace children are edges.
#[derive(Debug, Default)]
pub struct Dependencies {
    graph: DependencyGraph,
    node_map: HashMap<EntityId, NodeIndex>,
}

impl Dependencies {
    /// Builds the dependency graph for `api`. Clears any existing data first.
    ///
    /// Important: this assumes the [Api] is already validated! If it is invalid, panics may occur.
    pub fn build(&mut self, api: &Api) {
        self.graph.clear();
        self.node_map.clear();
        // Add in two passes: all nodes, then all edges. When adding an edge for a relative path,
        // we need to be able to resolve the entity id to a fully qualified path.
        self.add_nodes_recursively(api, &EntityId::default());
        self.add_edges_recursively(api, &EntityId::default());
    }

    pub fn contains_node(&self, entity_id: &EntityId) -> bool {
        self.node_map.contains_key(entity_id)
    }

    pub fn contains_edge(&self, from: &EntityId, to: &EntityId) -> bool {
        match (self.node_map.get(from), self.node_map.get(to)) {
            (Some(from), Some(to)) => self.graph.contains_edge(*from, *to),
            _ => false,
        }
    }

    fn add_nodes_recursively(&mut self, namespace: &Namespace, namespace_id: &EntityId) {
        for dto in namespace.dtos() {
            self.add_node(&namespace_id.child(dto.name));
        }

        for rpc in namespace.rpcs() {
            self.add_node(&namespace_id.child(rpc.name));
        }

        for en in namespace.enums() {
            self.add_node(&namespace_id.child(en.name));
        }

        for child in namespace.namespaces() {
            self.add_nodes_recursively(child, &namespace_id.child(&child.name));
        }
    }

    fn add_edges_recursively(&mut self, namespace: &Namespace, namespace_id: &EntityId) {
        for dto in namespace.dtos() {
            let from = *self.node(&namespace_id.child(dto.name)).unwrap();
            for field in &dto.fields {
                self.add_edge(from, namespace_id, &field.ty);
            }
        }

        for rpc in namespace.rpcs() {
            let from = *self.node(&namespace_id.child(rpc.name)).unwrap();
            for param in &rpc.params {
                self.add_edge(from, namespace_id, &param.ty);
            }
            if let Some(return_type) = &rpc.return_type {
                self.add_edge(from, namespace_id, &return_type);
            }
        }

        for child in namespace.namespaces() {
            self.add_edges_recursively(child, &namespace_id.child(&child.name));
        }
    }

    fn add_node(&mut self, entity_id: &EntityId) -> NodeIndex {
        let index = self.graph.add_node(entity_id.clone());
        self.node_map.insert(entity_id.clone(), index);
        index
    }

    fn node(&self, entity_id: &EntityId) -> Option<&NodeIndex> {
        self.node_map.get(entity_id)
    }

    fn node_relative(&self, base: &EntityId, relative: &EntityId) -> Option<&NodeIndex> {
        let mut it = Some(base.clone());
        while let Some(base) = it {
            let entity_id = base.concat(relative);
            if let Some(index) = self.node_map.get(&entity_id) {
                return Some(index);
            }
            it = base.parent();
        }
        None
    }

    fn add_edge(&mut self, from: NodeIndex, namespace_id: &EntityId, ty: &Type) {
        match ty {
            Type::Bool
            | Type::U8
            | Type::U16
            | Type::U32
            | Type::U64
            | Type::U128
            | Type::I8
            | Type::I16
            | Type::I32
            | Type::I64
            | Type::I128
            | Type::F8
            | Type::F16
            | Type::F32
            | Type::F64
            | Type::F128
            | Type::String
            | Type::Bytes
            | Type::User(_) => return,

            Type::Api(entity_id) => self.add_edge_relative(from, namespace_id, entity_id),

            Type::Array(ty) | Type::Optional(ty) => self.add_edge(from, namespace_id, ty),

            Type::Map { key, value } => {
                self.add_edge(from, namespace_id, key);
                self.add_edge(from, namespace_id, value);
            }
        }
    }

    fn add_edge_relative(
        &mut self,
        from: NodeIndex,
        namespace_id: &EntityId,
        relative_id: &EntityId,
    ) {
        // We unwrap nodes here because we assume the api is validated, and all nodes are added first.
        let to = self.node_relative(namespace_id, relative_id).unwrap();
        self.graph.add_edge(from, *to, ());
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{Api, Dependencies};
    use crate::test_util::executor::TestExecutor;

    mod contains_node {
        use crate::model::api::dependencies::tests::run_test;
        use crate::model::EntityId;

        #[test]
        fn success() {
            let node_id = EntityId::from("dto");
            run_test(r#"struct dto {}"#, |deps| {
                assert!(deps.contains_node(&node_id))
            });
        }

        #[test]
        fn failure() {
            let node_id = EntityId::from("rpc");
            run_test(r#"struct dto {}"#, |deps| {
                assert!(!deps.contains_node(&node_id))
            });
        }
    }

    mod contains_edge {
        use crate::model::api::dependencies::tests::run_test;
        use crate::model::EntityId;

        #[test]
        fn sibling() {
            let from = EntityId::from("dto1");
            let to = EntityId::from("dto0");
            run_test(
                r#"
            struct dto0 {}
            struct dto1 {
                field: dto0,
            }
            "#,
                |deps| assert!(deps.contains_edge(&from, &to)),
            );
        }

        #[test]
        fn parent() {
            let from = EntityId::from("ns.dto1");
            let to = EntityId::from("dto0");
            run_test(
                r#"
            struct dto0 {}
            mod ns {
                struct dto1 {
                    field: dto0,
                }
            }
            "#,
                |deps| assert!(deps.contains_edge(&from, &to)),
            );
        }

        #[test]
        fn nephew() {
            let from = EntityId::from("dto1");
            let to = EntityId::from("ns.dto0");
            run_test(
                r#"
            mod ns {
                struct dto0 {}
            }
            struct dto1 {
                field: ns::dto0,
            }
            "#,
                |deps| assert!(deps.contains_edge(&from, &to)),
            );
        }

        #[test]
        fn cousin() {
            let from = EntityId::from("ns1.dto1");
            let to = EntityId::from("ns0.dto0");
            run_test(
                r#"
            mod ns0 {
                struct dto0 {}
            }
            mod ns1 {
                struct dto1 {
                    field: ns0::dto0,
                }
            }
            "#,
                |deps| assert!(deps.contains_edge(&from, &to)),
            );
        }

        #[test]
        fn always_fully_qualified() {
            let a = EntityId::from("ns0.ns1.dto1");
            let b = EntityId::from("ns0.dto0");
            run_test(
                r#"
            mod ns0 {
                struct dto0 {
                    field: ns1::dto1
                }
                mod ns1 {
                    struct dto1 {
                        field: dto0,
                    }
                }
            }
            "#,
                |deps| {
                    assert!(deps.contains_edge(&a, &b));
                    assert!(deps.contains_edge(&b, &a));
                },
            );
        }

        #[test]
        fn failure() {
            let from = EntityId::from("dto0");
            let to = EntityId::from("dto1");
            run_test(
                r#"
            struct dto0 {}
            "#,
                |deps| assert!(!deps.contains_edge(&from, &to)),
            );
        }
    }

    mod adds_nodes_for_each {
        use crate::model::api::dependencies::tests::run_test;
        use crate::model::EntityId;

        #[test]
        fn dto() {
            run_test(
                r#"
            struct dto {}
            mod ns0 {
                struct dto {}
                mod ns1 {
                    struct dto {}
                }
            }
            "#,
                |deps| {
                    assert!(deps.node(&EntityId::from("dto")).is_some());
                    assert!(deps.node(&EntityId::from("ns0.dto")).is_some());
                    assert!(deps.node(&EntityId::from("ns0.ns1.dto")).is_some());
                    assert_eq!(deps.graph.node_count(), 3);
                },
            );
        }

        #[test]
        fn rpc() {
            run_test(
                r#"
            fn rpc() {}
            mod ns0 {
                fn rpc() {}
                mod ns1 {
                    fn rpc() {}
                }
            }
            "#,
                |deps| {
                    assert!(deps.node(&EntityId::from("rpc")).is_some());
                    assert!(deps.node(&EntityId::from("ns0.rpc")).is_some());
                    assert!(deps.node(&EntityId::from("ns0.ns1.rpc")).is_some());
                    assert_eq!(deps.graph.node_count(), 3);
                },
            );
        }

        #[test]
        fn en() {
            run_test(
                r#"
            enum en {}
            mod ns0 {
                enum en {}
                mod ns1 {
                    enum en {}
                }
            }
            "#,
                |deps| {
                    assert!(deps.node(&EntityId::from("en")).is_some());
                    assert!(deps.node(&EntityId::from("ns0.en")).is_some());
                    assert!(deps.node(&EntityId::from("ns0.ns1.en")).is_some());
                    assert_eq!(deps.graph.node_count(), 3);
                },
            );
        }
    }

    mod adds_edges_for {
        use crate::model::api::dependencies::tests::run_test;

        #[test]
        fn dto_field_types() {
            run_test(
                r#"
            struct dto0 {
                field: dto1,
            }
            struct dto1 {
                field: dto0,
            }
            "#,
                |deps| {
                    assert_eq!(deps.graph.edge_count(), 2);
                },
            );
        }

        #[test]
        fn rpc_param_types() {
            run_test(
                r#"
            struct dto0 {}
            struct dto1 {}
            fn rpc(param: dto0, param: dto1) {}
            "#,
                |deps| {
                    assert_eq!(deps.graph.edge_count(), 2);
                },
            );
        }

        #[test]
        fn rpc_return_types() {
            run_test(
                r#"
            struct dto0 {}
            fn rpc() -> dto0 {}
            "#,
                |deps| {
                    assert_eq!(deps.graph.edge_count(), 1);
                },
            );
        }
    }

    mod complex_types {
        use crate::model::api::dependencies::tests::run_test;
        use crate::model::EntityId;

        #[test]
        fn array() {
            run_complex_type_test(
                r#"
                mod ns {
                    struct src {
                        field: Vec<dto>,
                        field: Vec<en>,
                    }
                    struct dto {}
                    enum en {}
                }
                "#,
            );
        }

        #[test]
        fn optional() {
            run_complex_type_test(
                r#"
                mod ns {
                    struct src {
                        field: Option<dto>,
                        field: Option<en>,
                    }
                    struct dto {}
                    enum en {}
                }
                "#,
            );
        }

        #[test]
        fn map() {
            run_complex_type_test(
                r#"
                mod ns {
                    struct src {
                        field: HashMap<en, dto>,
                    }
                    struct dto {}
                    enum en {}
                }
                "#,
            );
        }

        #[test]
        fn transitive() {
            run_complex_type_test(
                r#"
                mod ns {
                    struct src {
                        field: Option<Vec<HashMap<en, dto>>>,
                    }
                    struct dto {}
                    enum en {}
                }
            "#,
            );
        }

        fn run_complex_type_test(data: &str) {
            let src = EntityId::from("ns.src");
            let dto = EntityId::from("ns.dto");
            let en = EntityId::from("ns.en");
            run_test(data, |deps| {
                assert!(deps.contains_edge(&src, &dto));
                assert!(deps.contains_edge(&src, &en));
            });
        }
    }

    #[test]
    fn clears_existing_on_build() {
        let mut exe = TestExecutor::new("struct dto {} fn rpc() {}");
        let api = exe.api();
        let mut dependencies = Dependencies::default();
        dependencies.build(&api);
        assert_eq!(dependencies.graph.node_count(), 2);
        dependencies.build(&Api::default());
        assert_eq!(dependencies.graph.node_count(), 0);
    }

    fn run_test<F: Fn(&Dependencies)>(data: &str, f: F) {
        let mut exe = TestExecutor::new(data);
        let api = exe.api();
        let mut dependencies = Dependencies::default();
        dependencies.build(&api);
        println!("MAP: {:#?} ", dependencies.node_map);
        println!("GRAPH: {:#?} ", dependencies.graph);
        f(&dependencies)
    }
}
