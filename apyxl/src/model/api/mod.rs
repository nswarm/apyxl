pub use builder::Builder;
use itertools::Itertools;

mod builder;

/// A complete set of components that make up an API. The root [Namespace] of the entire API.
/// The name will always be [UNDEFINED_NAMESPACE]
pub type Api<'a> = Namespace<'a>;

/// The root namespace of the entire API.
pub const UNDEFINED_NAMESPACE: &str = "_";

#[derive(Debug, Eq, PartialEq)]
pub enum NamespaceChild<'a> {
    Dto(Dto<'a>),
    Rpc(Rpc<'a>),
    Namespace(Namespace<'a>),
}

/// A named, nestable wrapper for a set of API components.
#[derive(Default, Debug, Eq, PartialEq)]
pub struct Namespace<'a> {
    pub name: &'a str,
    pub children: Vec<NamespaceChild<'a>>,
}

/// A single Data Transfer Object (DTO) used in an [Rpc], either directly or nested in another [Dto].
#[derive(Default, Debug, Eq, PartialEq)]
pub struct Dto<'a> {
    pub name: &'a str,
    pub fields: Vec<Field<'a>>,
}

/// A pair of name and type that describe a named instance of a type e.g. within a [Dto] or [Rpc].
#[derive(Default, Debug, Eq, PartialEq)]
pub struct Field<'a> {
    pub name: &'a str,
    pub ty: TypeRef<'a>,
}

/// A single Remote Procedure Call (RPC) within an [Api].
#[derive(Default, Debug, Eq, PartialEq)]
pub struct Rpc<'a> {
    pub name: &'a str,
    pub params: Vec<Field<'a>>,
    pub return_type: Option<TypeRef<'a>>,
}

/// A type such as a language primitive or a reference to a [Dto]. A [Dto] reference will contain
/// all necessary information to find the exact [Dto] within the API.
#[derive(Default, Debug, Eq, PartialEq, Clone)]
pub struct TypeRef<'a> {
    pub fully_qualified_type_name: Vec<&'a str>,
}

impl<'a> TypeRef<'a> {
    pub fn new(fqtn: &[&'a str]) -> Self {
        Self {
            fully_qualified_type_name: fqtn.to_vec(),
        }
    }

    pub fn child(&self, name: &'a str) -> Self {
        let mut child = self.clone();
        child.fully_qualified_type_name.push(name);
        child
    }

    pub fn has_namespace(&self) -> bool {
        self.fully_qualified_type_name.len() > 1
    }

    /// Returns an iterator over the part of the path _before_ the name, which represents the
    /// namespace it is a part of as an iterator over the type ref.
    pub fn namespace_iter<'b>(&'b self) -> impl Iterator<Item = &'a str> + 'b {
        let len = self.fully_qualified_type_name.len();
        self.fully_qualified_type_name.iter().copied().take(len - 1)
    }

    /// Returns the part of the path _before_ the name
    pub fn namespace(&self) -> TypeRef<'a> {
        TypeRef::new(&self.namespace_iter().collect::<Vec<_>>())
    }

    /// The name is always the last part of the type path.
    pub fn name(&self) -> Option<&'a str> {
        self.fully_qualified_type_name.last().copied()
    }
}

impl<'a> From<&[&'a str]> for TypeRef<'a> {
    fn from(value: &[&'a str]) -> Self {
        Self {
            fully_qualified_type_name: value.to_vec(),
        }
    }
}

impl<'a> From<Vec<&'a str>> for TypeRef<'a> {
    fn from(value: Vec<&'a str>) -> Self {
        Self {
            fully_qualified_type_name: value,
        }
    }
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
    fn dto(&self, name: &str) -> Option<&Dto<'a>> {
        self.children.iter().find_map(|s| match s {
            NamespaceChild::Dto(value) if value.name == name => Some(value),
            _ => None,
        })
    }

    /// Get a [Dto] within this [Namespace] by name.
    fn dto_mut(&mut self, name: &str) -> Option<&mut Dto<'a>> {
        self.children.iter_mut().find_map(|s| match s {
            NamespaceChild::Dto(value) if value.name == name => Some(value),
            _ => None,
        })
    }

    /// Get a [Rpc] within this [Namespace] by name.
    fn rpc(&self, name: &str) -> Option<&Rpc<'a>> {
        self.children.iter().find_map(|s| match s {
            NamespaceChild::Rpc(value) if value.name == name => Some(value),
            _ => None,
        })
    }

    /// Get a [Rpc] within this [Namespace] by name.
    fn rpc_mut(&mut self, name: &str) -> Option<&mut Rpc<'a>> {
        self.children.iter_mut().find_map(|s| match s {
            NamespaceChild::Rpc(value) if value.name == name => Some(value),
            _ => None,
        })
    }

    /// Get a [Namespace] within this [Namespace] by name.
    fn namespace(&self, name: &str) -> Option<&Namespace<'a>> {
        self.children.iter().find_map(|s| match s {
            NamespaceChild::Namespace(value) if value.name == name => Some(value),
            _ => None,
        })
    }

    /// Get a [Namespace] within this [Namespace] by name.
    fn namespace_mut(&mut self, name: &str) -> Option<&mut Namespace<'a>> {
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

    /// Find a [Dto] by `type_ref` relative to this [Namespace].
    pub fn find_dto(&self, type_ref: &TypeRef) -> Option<&Dto<'a>> {
        if !type_ref.has_namespace() {
            return type_ref.name().and_then(|name| self.dto(name));
        }
        let namespace = self.find_namespace(&type_ref.namespace());
        let name = type_ref.name();
        match (namespace, name) {
            (Some(namespace), Some(name)) => namespace.dto(name),
            _ => None,
        }
    }

    /// Find a [Dto] by `type_ref` relative to this [Namespace].
    pub fn find_dto_mut(&mut self, type_ref: &TypeRef) -> Option<&mut Dto<'a>> {
        if !type_ref.has_namespace() {
            return type_ref.name().and_then(|name| self.dto_mut(name));
        }
        let namespace = self.find_namespace_mut(&type_ref.namespace());
        let name = type_ref.name();
        match (namespace, name) {
            (Some(namespace), Some(name)) => namespace.dto_mut(name),
            _ => None,
        }
    }

    /// Find a [Rpc] by `type_ref` relative to this [Namespace].
    pub fn find_rpc(&self, type_ref: &TypeRef) -> Option<&Rpc<'a>> {
        let namespace = self.find_namespace(&type_ref.namespace());
        let name = type_ref.name();
        match (namespace, name) {
            (Some(namespace), Some(name)) => namespace.rpc(name),
            _ => None,
        }
    }

    /// Find a [Rpc] by `type_ref` relative to this [Namespace].
    pub fn find_rpc_mut(&mut self, type_ref: &TypeRef) -> Option<&mut Rpc<'a>> {
        let namespace = self.find_namespace_mut(&type_ref.namespace());
        let name = type_ref.name();
        match (namespace, name) {
            (Some(namespace), Some(name)) => namespace.rpc_mut(name),
            _ => None,
        }
    }

    /// Find a [Namespace] by `type_ref` relative to this [Namespace].
    /// If the type ref is empty, this [Namespace] will be returned.
    pub fn find_namespace(&self, type_ref: &TypeRef) -> Option<&Namespace<'a>> {
        let mut namespace_it = self;
        for name in &type_ref.fully_qualified_type_name {
            if let Some(namespace) = namespace_it.namespace(name) {
                namespace_it = namespace;
            } else {
                return None;
            }
        }
        Some(namespace_it)
    }

    /// Find a [Namespace] by `type_ref` relative to this [Namespace].
    pub fn find_namespace_mut(&mut self, type_ref: &TypeRef) -> Option<&mut Namespace<'a>> {
        let mut namespace_it = self;
        for name in &type_ref.fully_qualified_type_name {
            if let Some(namespace) = namespace_it.namespace_mut(name) {
                namespace_it = namespace;
            } else {
                return None;
            }
        }
        Some(namespace_it)
    }
}

#[cfg(test)]
pub mod tests {
    use crate::model::{Api, Dto, Namespace, Rpc};

    #[test]
    fn merge() {
        let mut ns0 = test_namespace(1);
        ns0.add_rpc(test_rpc(1));
        ns0.add_dto(test_dto(1));
        ns0.add_namespace(test_namespace(3));

        let mut ns1 = test_namespace(2);
        ns1.add_rpc(test_rpc(2));
        ns1.add_dto(test_dto(2));
        ns1.add_namespace(test_namespace(4));

        ns0.merge(ns1);
        assert_eq!(ns0.dtos().count(), 2);
        assert_eq!(ns0.rpcs().count(), 2);
        assert_eq!(ns0.namespaces().count(), 2);
        assert!(ns0.dto(test_dto(1).name).is_some());
        assert!(ns0.dto(test_dto(2).name).is_some());
        assert!(ns0.dto(test_rpc(1).name).is_some());
        assert!(ns0.dto(test_rpc(2).name).is_some());
        assert!(ns0.dto(test_namespace(3).name).is_some());
        assert!(ns0.dto(test_namespace(4).name).is_some());
    }

    mod add_get {
        use crate::model::api::tests::{complex_api, complex_namespace, test_dto, test_rpc, NAMES};

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
        use crate::model::api::tests::{complex_api, complex_namespace, test_dto, test_rpc};

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
        use crate::model::api::tests::{
            complex_api, complex_namespace, test_dto, test_namespace, test_rpc, NAMES,
        };
        use crate::model::TypeRef;

        #[test]
        fn dto() {
            let mut api = complex_api();
            let type_ref1 = TypeRef::new(&[test_dto(1).name]);
            let type_ref2 = TypeRef::new(&[test_dto(2).name]);
            assert_eq!(api.find_dto(&type_ref1), Some(&test_dto(1)));
            assert_eq!(api.find_dto_mut(&type_ref2), Some(&mut test_dto(2)));
        }

        #[test]
        fn rpc() {
            let mut api = complex_api();
            let type_ref1 = TypeRef::new(&[test_dto(1).name]);
            let type_ref2 = TypeRef::new(&[test_dto(2).name]);
            assert_eq!(api.find_rpc(&type_ref1), Some(&test_rpc(1)),);
            assert_eq!(api.find_rpc_mut(&type_ref2), Some(&mut test_rpc(2)),);
        }

        #[test]
        fn namespace() {
            let mut api = complex_api();
            let type_ref1 = TypeRef::new(&[complex_namespace(1).name]);
            let type_ref2 = TypeRef::new(&[complex_namespace(2).name]);
            assert_eq!(api.find_namespace(&type_ref1), Some(&complex_namespace(1)));
            assert_eq!(
                api.find_namespace_mut(&type_ref2),
                Some(&mut complex_namespace(2))
            );
        }

        #[test]
        fn child() {
            let api = complex_api();
            let type_ref = TypeRef::new(&[complex_namespace(1).name, NAMES[3]]);
            assert_eq!(api.find_dto(&type_ref), Some(&test_dto(3)));
            assert_eq!(api.find_rpc(&type_ref), Some(&test_rpc(3)));
            assert_eq!(api.find_namespace(&type_ref), Some(&test_namespace(3)));
        }

        #[test]
        fn multi_depth_child() {
            let api = complex_api();
            let type_ref =
                TypeRef::new(&[complex_namespace(1).name, test_namespace(4).name, NAMES[5]]);
            assert_eq!(api.find_dto(&type_ref), Some(&test_dto(5)));
        }
    }

    const NAMES: &[&str] = &["name0", "name1", "name2", "name3", "name4", "name5"];

    pub fn complex_api() -> Api<'static> {
        let mut api = Api::default();
        api.add_dto(test_dto(1));
        api.add_dto(test_dto(2));
        api.add_rpc(test_rpc(1));
        api.add_rpc(test_rpc(2));
        api.add_namespace(complex_namespace(1));
        api.add_namespace(complex_namespace(2));
        api
    }

    pub fn complex_namespace(i: usize) -> Namespace<'static> {
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

    pub fn test_dto(i: usize) -> Dto<'static> {
        Dto {
            name: NAMES[i],
            ..Default::default()
        }
    }

    pub fn test_rpc(i: usize) -> Rpc<'static> {
        Rpc {
            name: NAMES[i],
            ..Default::default()
        }
    }

    pub fn test_namespace(i: usize) -> Namespace<'static> {
        Namespace {
            name: NAMES[i],
            ..Default::default()
        }
    }
}
