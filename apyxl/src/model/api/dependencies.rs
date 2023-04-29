use crate::model::{Api, EntityId, Namespace};
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
    pub fn build(&mut self, api: &Api) {
        self.graph.clear();
        self.node_map.clear();
        self.add_dependencies_recursively(api, &EntityId::default());
    }

    fn add_dependencies_recursively(&mut self, namespace: &Namespace, entity_id: &EntityId) {
        for dto in namespace.dtos() {
            let dto_id = entity_id.child(dto.name);
            let dto_node_index = self.add_node(&dto_id);
            for field in &dto.fields {
                self.add_edge(dto_node_index, &field.ty);
            }
        }

        for rpc in namespace.rpcs() {}

        for child in namespace.namespaces() {
            self.add_dependencies_recursively(child, &entity_id.child(&child.name));
        }
    }

    fn add_node(&mut self, entity_id: &EntityId) -> NodeIndex {
        let index = self.graph.add_node(entity_id.clone());
        self.node_map.insert(entity_id.clone(), index);
        index
    }

    fn add_edge(&mut self, from_index: NodeIndex, to: &EntityId) {
        let to_index = self.get_or_add_node(to);
        self.graph.add_edge(from_index, to_index, ());
    }

    fn get_or_add_node(&mut self, entity_id: &EntityId) -> NodeIndex {
        match self.node_map.get(&entity_id) {
            None => self.add_node(&entity_id),
            Some(index) => index.clone(),
        }
    }
}
