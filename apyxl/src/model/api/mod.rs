use std::borrow::Cow;

use itertools::Itertools;

pub use dependencies::Dependencies;
pub use entity_id::EntityId;
pub use validate::ValidationError;

use crate::model::chunk;

mod dependencies;
mod entity_id;
pub mod validate;

/// A complete set of entities that make up an API.
pub type Api<'a> = Namespace<'a>;

/// The root namespace of the entire API.
pub const UNDEFINED_NAMESPACE: &str = "_";

/// A named, nestable wrapper for a set of API entities.
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct Namespace<'a> {
    pub name: Cow<'a, str>,
    pub children: Vec<NamespaceChild<'a>>,
    pub attributes: Attributes,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum NamespaceChild<'a> {
    Dto(Dto<'a>),
    Rpc(Rpc<'a>),
    Namespace(Namespace<'a>),
}

/// A single Data Transfer Object (DTO) used in an [Rpc], either directly or nested in another [Dto].
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct Dto<'a> {
    pub name: &'a str,
    pub fields: Vec<Field<'a>>,
    pub attributes: Attributes,
}

/// A pair of name and type that describe a named instance of a type e.g. within a [Dto] or [Rpc].
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct Field<'a> {
    pub name: &'a str,
    pub ty: EntityId,
    pub attributes: Attributes,
}

/// A single Remote Procedure Call (RPC) within an [Api].
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct Rpc<'a> {
    pub name: &'a str,
    pub params: Vec<Field<'a>>,
    pub return_type: Option<EntityId>,
    pub attributes: Attributes,
}

/// Additional metadata attached to entities.
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct Attributes {
    pub chunk: Option<chunk::Attribute>,
}

impl<'a> Namespace<'a> {
    /// Perform a simple merge of [Namespace] `other` into this [Namespace] by adding all of
    /// `other`'s children to to this [Namespace]'s children. `other`'s name is ignored. This may
    /// result in duplicate children.
    pub fn merge(&mut self, mut other: Namespace<'a>) {
        self.children.append(&mut other.children)
    }

    /// Add dto [Dto] `dto` as a child of this [Namespace].
    /// No validation is performed to ensure the [Dto] does not already exist, which may result
    /// in duplicates.
    pub fn add_dto(&mut self, dto: Dto<'a>) {
        self.children.push(NamespaceChild::Dto(dto));
    }

    /// Add the [Rpc] `rpc` as a child of this [Namespace].
    /// No validation is performed to ensure the [Rpc] does not already exist, which may result
    //     /// in duplicates.
    pub fn add_rpc(&mut self, rpc: Rpc<'a>) {
        self.children.push(NamespaceChild::Rpc(rpc));
    }

    /// Add the [Namespace] `namespace` as a child of this [Namespace].
    /// No validation is performed to ensure the [Namespace] does not already exist, which may result
    //     /// in duplicates.
    pub fn add_namespace(&mut self, namespace: Namespace<'a>) {
        self.children.push(NamespaceChild::Namespace(namespace));
    }

    /// Get a [Dto] within this [Namespace] by name.
    pub fn dto(&self, name: &str) -> Option<&Dto<'a>> {
        self.children.iter().find_map(|s| match s {
            NamespaceChild::Dto(value) if value.name == name => Some(value),
            _ => None,
        })
    }

    /// Get a [Dto] within this [Namespace] by name.
    pub fn dto_mut(&mut self, name: &str) -> Option<&mut Dto<'a>> {
        self.children.iter_mut().find_map(|s| match s {
            NamespaceChild::Dto(value) if value.name == name => Some(value),
            _ => None,
        })
    }

    /// Get a [Rpc] within this [Namespace] by name.
    pub fn rpc(&self, name: &str) -> Option<&Rpc<'a>> {
        self.children.iter().find_map(|s| match s {
            NamespaceChild::Rpc(value) if value.name == name => Some(value),
            _ => None,
        })
    }

    /// Get a [Rpc] within this [Namespace] by name.
    pub fn rpc_mut(&mut self, name: &str) -> Option<&mut Rpc<'a>> {
        self.children.iter_mut().find_map(|s| match s {
            NamespaceChild::Rpc(value) if value.name == name => Some(value),
            _ => None,
        })
    }

    /// Get a [Namespace] within this [Namespace] by name.
    pub fn namespace(&self, name: &str) -> Option<&Namespace<'a>> {
        self.children.iter().find_map(|s| match s {
            NamespaceChild::Namespace(value) if value.name == name => Some(value),
            _ => None,
        })
    }

    /// Get a [Namespace] within this [Namespace] by name.
    pub fn namespace_mut(&mut self, name: &str) -> Option<&mut Namespace<'a>> {
        self.children.iter_mut().find_map(|s| match s {
            NamespaceChild::Namespace(value) if value.name == name => Some(value),
            _ => None,
        })
    }

    /// Iterate over all [Dto]s within this [Namespace].
    pub fn dtos(&self) -> impl Iterator<Item = &Dto<'a>> {
        self.children.iter().filter_map(|child| {
            if let NamespaceChild::Dto(value) = child {
                Some(value)
            } else {
                None
            }
        })
    }

    /// Iterate over all [Rpc]s within this [Namespace].
    pub fn rpcs(&self) -> impl Iterator<Item = &Rpc<'a>> {
        self.children.iter().filter_map(|child| {
            if let NamespaceChild::Rpc(value) = child {
                Some(value)
            } else {
                None
            }
        })
    }

    /// Iterate over all [Namespace]s within this [Namespace].
    pub fn namespaces(&self) -> impl Iterator<Item = &Namespace<'a>> {
        self.children.iter().filter_map(|child| {
            if let NamespaceChild::Namespace(value) = child {
                Some(value)
            } else {
                None
            }
        })
    }

    /// Iterate mutably over all [Namespace]s within this [Namespace].
    pub fn namespaces_mut(&mut self) -> impl Iterator<Item = &mut Namespace<'a>> {
        self.children.iter_mut().filter_map(|child| {
            if let NamespaceChild::Namespace(value) = child {
                Some(value)
            } else {
                None
            }
        })
    }

    /// Removes all [Namespaces] from this [Namespace] and returns them in a [Vec].
    pub fn take_namespaces(&mut self) -> Vec<Namespace<'a>> {
        self.children
            .drain_filter(|child| matches!(child, NamespaceChild::Namespace(_)))
            .map(|child| {
                if let NamespaceChild::Namespace(ns) = child {
                    ns
                } else {
                    unreachable!("already checked that it matches")
                }
            })
            .collect_vec()
    }

    /// Find a [Dto] by [EntityId] relative to this [Namespace].
    pub fn find_dto(&self, entity_id: &EntityId) -> Option<&Dto<'a>> {
        if !entity_id.has_namespace() {
            return entity_id.name().and_then(|name| self.dto(&name));
        }
        let namespace = self.find_namespace(&entity_id.namespace());
        let name = entity_id.name();
        match (namespace, name) {
            (Some(namespace), Some(name)) => namespace.dto(&name),
            _ => None,
        }
    }

    /// Find a [Dto] by [EntityId] relative to this [Namespace].
    pub fn find_dto_mut(&mut self, entity_id: &EntityId) -> Option<&mut Dto<'a>> {
        if !entity_id.has_namespace() {
            return entity_id.name().and_then(|name| self.dto_mut(&name));
        }
        let namespace = self.find_namespace_mut(&entity_id.namespace());
        let name = entity_id.name();
        match (namespace, name) {
            (Some(namespace), Some(name)) => namespace.dto_mut(&name),
            _ => None,
        }
    }

    /// Find a [Rpc] by [EntityId] relative to this [Namespace].
    pub fn find_rpc(&self, entity_id: &EntityId) -> Option<&Rpc<'a>> {
        let namespace = self.find_namespace(&entity_id.namespace());
        let name = entity_id.name();
        match (namespace, name) {
            (Some(namespace), Some(name)) => namespace.rpc(&name),
            _ => None,
        }
    }

    /// Find a [Rpc] by [EntityId] relative to this [Namespace].
    pub fn find_rpc_mut(&mut self, entity_id: &EntityId) -> Option<&mut Rpc<'a>> {
        let namespace = self.find_namespace_mut(&entity_id.namespace());
        let name = entity_id.name();
        match (namespace, name) {
            (Some(namespace), Some(name)) => namespace.rpc_mut(&name),
            _ => None,
        }
    }

    /// Find a [Namespace] by [EntityId] relative to this [Namespace].
    /// If the type ref is empty, this [Namespace] will be returned.
    pub fn find_namespace(&self, entity_id: &EntityId) -> Option<&Namespace<'a>> {
        let mut namespace_it = self;
        for name in &entity_id.path {
            if let Some(namespace) = namespace_it.namespace(name) {
                namespace_it = namespace;
            } else {
                return None;
            }
        }
        Some(namespace_it)
    }

    /// Find a [Namespace] by [EntityId] relative to this [Namespace].
    pub fn find_namespace_mut(&mut self, entity_id: &EntityId) -> Option<&mut Namespace<'a>> {
        let mut namespace_it = self;
        for name in &entity_id.path {
            if let Some(namespace) = namespace_it.namespace_mut(name) {
                namespace_it = namespace;
            } else {
                return None;
            }
        }
        Some(namespace_it)
    }

    pub fn apply_attr_to_children_recursively<F: FnMut(&mut Attributes) + Clone>(
        &mut self,
        mut f: F,
    ) {
        for namespace in self.namespaces_mut() {
            namespace.apply_attr_to_children_recursively(f.clone());
        }
        for child in &mut self.children {
            f(child.attributes_mut())
        }
    }
}

impl<'a> NamespaceChild<'a> {
    pub fn attributes(&self) -> &Attributes {
        match self {
            NamespaceChild::Dto(dto) => &dto.attributes,
            NamespaceChild::Rpc(rpc) => &rpc.attributes,
            NamespaceChild::Namespace(namespace) => &namespace.attributes,
        }
    }

    pub fn attributes_mut(&mut self) -> &mut Attributes {
        match self {
            NamespaceChild::Dto(dto) => &mut dto.attributes,
            NamespaceChild::Rpc(rpc) => &mut rpc.attributes,
            NamespaceChild::Namespace(namespace) => &mut namespace.attributes,
        }
    }
}

impl<'a> Dto<'a> {
    pub fn field(&self, name: &str) -> Option<&Field<'a>> {
        self.fields.iter().find(|field| field.name == name)
    }

    pub fn field_mut(&mut self, name: &str) -> Option<&mut Field<'a>> {
        self.fields.iter_mut().find(|field| field.name == name)
    }
}

impl<'a> Rpc<'a> {
    pub fn param(&self, name: &str) -> Option<&Field<'a>> {
        self.params.iter().find(|param| param.name == name)
    }

    pub fn param_mut(&mut self, name: &str) -> Option<&mut Field<'a>> {
        self.params.iter_mut().find(|param| param.name == name)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::model::{chunk, Api, EntityId, Namespace};
    use crate::test_util::executor::TestExecutor;
    use crate::test_util::{test_dto, test_namespace, test_rpc};

    #[test]
    fn merge() {
        let mut exe0 = TestExecutor::new(
            r#"
            fn rpc0() {}
            struct dto0 {}
            mod nested0 {}
        "#,
        );
        let mut ns0 = exe0.api();

        let mut exe1 = TestExecutor::new(
            r#"
            fn rpc1() {}
            struct dto1 {}
            mod nested1 {}
        "#,
        );
        let ns1 = exe1.api();

        ns0.merge(ns1);
        assert_eq!(ns0.dtos().count(), 2);
        assert_eq!(ns0.rpcs().count(), 2);
        assert_eq!(ns0.namespaces().count(), 2);
        assert!(ns0.dto("dto0").is_some());
        assert!(ns0.dto("dto1").is_some());
        assert!(ns0.rpc("rpc0").is_some());
        assert!(ns0.rpc("rpc1").is_some());
        assert!(ns0.namespace("nested0").is_some());
        assert!(ns0.namespace("nested1").is_some());
    }

    mod add_get {
        use crate::model::api::tests::{complex_api, complex_namespace};
        use crate::test_util::{test_dto, test_rpc, NAMES};

        #[test]
        fn dto() {
            let mut api = complex_api();
            assert_eq!(api.dto(NAMES[1]), Some(test_dto(1)).as_ref());
            assert_eq!(api.dto(NAMES[2]), Some(test_dto(2)).as_ref());
            assert_eq!(api.dto_mut(NAMES[1]), Some(test_dto(1)).as_mut());
            assert_eq!(api.dto_mut(NAMES[2]), Some(test_dto(2)).as_mut());
        }

        #[test]
        fn rpc() {
            let mut api = complex_api();
            assert_eq!(api.rpc(NAMES[1]), Some(test_rpc(1)).as_ref());
            assert_eq!(api.rpc(NAMES[2]), Some(test_rpc(2)).as_ref());
            assert_eq!(api.rpc_mut(NAMES[1]), Some(test_rpc(1)).as_mut());
            assert_eq!(api.rpc_mut(NAMES[2]), Some(test_rpc(2)).as_mut());
        }

        #[test]
        fn namespace() {
            let mut api = complex_api();
            assert_eq!(api.namespace(NAMES[1]), Some(complex_namespace(1)).as_ref());
            assert_eq!(api.namespace(NAMES[2]), Some(complex_namespace(2)).as_ref());
            assert_eq!(
                api.namespace_mut(NAMES[1]),
                Some(complex_namespace(1)).as_mut()
            );
            assert_eq!(
                api.namespace_mut(NAMES[2]),
                Some(complex_namespace(2)).as_mut()
            );
        }
    }

    mod iter {
        use crate::model::api::tests::{complex_api, complex_namespace};
        use crate::test_util::{test_dto, test_rpc};

        #[test]
        fn dtos() {
            let api = complex_api();
            assert_eq!(
                api.dtos().collect::<Vec<_>>(),
                vec![&test_dto(1), &test_dto(2)]
            );
        }

        #[test]
        fn rpcs() {
            let api = complex_api();
            assert_eq!(
                api.rpcs().collect::<Vec<_>>(),
                vec![&test_rpc(1), &test_rpc(2)]
            );
        }

        #[test]
        fn namespaces() {
            let api = complex_api();
            assert_eq!(
                api.namespaces().collect::<Vec<_>>(),
                vec![&complex_namespace(1), &complex_namespace(2)]
            );
        }
    }

    mod find {
        use std::borrow::Cow;

        use crate::model::api::tests::{complex_api, complex_namespace};
        use crate::model::EntityId;
        use crate::test_util::{test_dto, test_namespace, test_rpc, NAMES};

        #[test]
        fn dto() {
            let mut api = complex_api();
            let entity_id1 = EntityId::new([test_dto(1).name]);
            let entity_id2 = EntityId::new([test_dto(2).name]);
            assert_eq!(api.find_dto(&entity_id1), Some(&test_dto(1)));
            assert_eq!(api.find_dto_mut(&entity_id2), Some(&mut test_dto(2)));
        }

        #[test]
        fn rpc() {
            let mut api = complex_api();
            let entity_id1 = EntityId::new([test_dto(1).name]);
            let entity_id2 = EntityId::new([test_dto(2).name]);
            assert_eq!(api.find_rpc(&entity_id1), Some(&test_rpc(1)),);
            assert_eq!(api.find_rpc_mut(&entity_id2), Some(&mut test_rpc(2)),);
        }

        #[test]
        fn namespace() {
            let mut api = complex_api();
            let entity_id1 = EntityId::new(&[complex_namespace(1).name]);
            let entity_id2 = EntityId::new(&[complex_namespace(2).name]);
            assert_eq!(api.find_namespace(&entity_id1), Some(&complex_namespace(1)));
            assert_eq!(
                api.find_namespace_mut(&entity_id2),
                Some(&mut complex_namespace(2))
            );
        }

        #[test]
        fn child() {
            let api = complex_api();
            let entity_id = EntityId::new(&[complex_namespace(1).name, Cow::Borrowed(NAMES[3])]);
            assert_eq!(api.find_dto(&entity_id), Some(&test_dto(3)));
            assert_eq!(api.find_rpc(&entity_id), Some(&test_rpc(3)));
            assert_eq!(api.find_namespace(&entity_id), Some(&test_namespace(3)));
        }

        #[test]
        fn multi_depth_child() {
            let api = complex_api();
            let entity_id = EntityId::new(&[
                complex_namespace(1).name,
                test_namespace(4).name,
                Cow::Borrowed(NAMES[5]),
            ]);
            assert_eq!(api.find_dto(&entity_id), Some(&test_dto(5)));
        }
    }

    mod parent {
        use crate::model::EntityId;

        #[test]
        fn no_parent() {
            let ty = EntityId::default();
            assert_eq!(ty.parent(), None);
        }

        #[test]
        fn parent_is_root() {
            let ty = EntityId::new(["dto"]);
            assert_eq!(ty.parent(), Some(EntityId::default()));
        }

        #[test]
        fn typical() {
            let ty = EntityId::new(["ns0", "ns1", "dto"]);
            let parent = ty.parent();
            assert_eq!(parent, Some(EntityId::new(["ns0", "ns1"])));
            assert_eq!(parent.unwrap().parent(), Some(EntityId::new(["ns0"])));
        }
    }

    #[test]
    fn apply_attr_to_children() {
        let mut exe = TestExecutor::new(
            r#"
                    mod ns0 {
                        mod ns1 {
                            struct dto {}
                            fn rpc() {}
                        }
                        struct dto {}
                        fn rpc() {}
                    }
                "#,
        );
        let mut api = exe.api();
        let expected_chunk = PathBuf::from("a/b/c");
        api.find_namespace_mut(&EntityId::new(["ns0"]))
            .unwrap()
            .apply_attr_to_children_recursively(|attr| {
                attr.chunk
                    .get_or_insert(chunk::Attribute::default())
                    .relative_file_paths
                    .push(expected_chunk.clone())
            });
        assert_eq!(
            api.find_namespace(&EntityId::new(["ns0", "ns1"]))
                .unwrap()
                .attributes
                .chunk
                .as_ref()
                .unwrap()
                .relative_file_paths,
            vec![expected_chunk.clone()]
        );
        assert_eq!(
            api.find_dto(&EntityId::new(["ns0", "dto"]))
                .unwrap()
                .attributes
                .chunk
                .as_ref()
                .unwrap()
                .relative_file_paths,
            vec![expected_chunk.clone()]
        );
        assert_eq!(
            api.find_rpc(&EntityId::new(["ns0", "rpc"]))
                .unwrap()
                .attributes
                .chunk
                .as_ref()
                .unwrap()
                .relative_file_paths,
            vec![expected_chunk.clone()]
        );
        assert_eq!(
            api.find_dto(&EntityId::new(["ns0", "ns1", "dto"]))
                .unwrap()
                .attributes
                .chunk
                .as_ref()
                .unwrap()
                .relative_file_paths,
            vec![expected_chunk.clone()]
        );
        assert_eq!(
            api.find_rpc(&EntityId::new(["ns0", "ns1", "rpc"]))
                .unwrap()
                .attributes
                .chunk
                .as_ref()
                .unwrap()
                .relative_file_paths,
            vec![expected_chunk.clone()]
        );
    }

    fn complex_api() -> Api<'static> {
        let mut api = Api::default();
        api.add_dto(test_dto(1));
        api.add_dto(test_dto(2));
        api.add_rpc(test_rpc(1));
        api.add_rpc(test_rpc(2));
        api.add_namespace(complex_namespace(1));
        api.add_namespace(complex_namespace(2));
        api
    }

    fn complex_namespace(i: usize) -> Namespace<'static> {
        let mut namespace = test_namespace(i);
        namespace.add_dto(test_dto(i + 2));
        namespace.add_dto(test_dto(i + 3));
        namespace.add_rpc(test_rpc(i + 2));
        namespace.add_rpc(test_rpc(i + 3));
        namespace.add_namespace(test_namespace(i + 2));
        let mut deep_namespace = test_namespace(i + 3);
        deep_namespace.add_dto(test_dto(5));
        namespace.add_namespace(deep_namespace);
        namespace
    }
}
