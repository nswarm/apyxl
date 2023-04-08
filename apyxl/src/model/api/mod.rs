use itertools::Itertools;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

pub use validate::ValidationError;

pub mod validate;

/// A complete set of entities that make up an API.
pub type Api<'a> = Namespace<'a>;

/// The root namespace of the entire API.
pub const UNDEFINED_NAMESPACE: &str = "_";

#[derive(Debug, Eq, PartialEq)]
pub enum NamespaceChild<'a> {
    Dto(Dto<'a>),
    Rpc(Rpc<'a>),
    Namespace(Namespace<'a>),
}

/// Arbitrary key=value pairs used to attach additional metadata to entities.
pub type Attributes<'a> = HashMap<&'a str, String>;

/// A named, nestable wrapper for a set of API entities.
#[derive(Default, Debug, Eq, PartialEq)]
pub struct Namespace<'a> {
    pub name: &'a str,
    pub children: Vec<NamespaceChild<'a>>,
    pub attributes: Attributes<'a>,
}

/// A single Data Transfer Object (DTO) used in an [Rpc], either directly or nested in another [Dto].
#[derive(Default, Debug, Eq, PartialEq)]
pub struct Dto<'a> {
    pub name: &'a str,
    pub fields: Vec<Field<'a>>,
    pub attributes: Attributes<'a>,
}

/// A pair of name and type that describe a named instance of a type e.g. within a [Dto] or [Rpc].
#[derive(Default, Debug, Eq, PartialEq)]
pub struct Field<'a> {
    pub name: &'a str,
    pub ty: TypeRef<'a>,
    pub attributes: Attributes<'a>,
}

/// A single Remote Procedure Call (RPC) within an [Api].
#[derive(Default, Debug, Eq, PartialEq)]
pub struct Rpc<'a> {
    pub name: &'a str,
    pub params: Vec<Field<'a>>,
    pub return_type: Option<TypeRef<'a>>,
    pub attributes: Attributes<'a>,
}

/// A type such as a language primitive or a reference to a type within the API. Typically when used
/// within model types, it refers to a [Dto], but can be used as a reference to other types
/// like [Rpc]s or [Namespace]s as well.
#[derive(Default, Debug, Eq, PartialEq, Clone, Hash)]
pub struct TypeRef<'a> {
    pub fully_qualified_type_name: Vec<&'a str>,
}

impl<'a> NamespaceChild<'a> {
    pub fn attributes(&self) -> &Attributes<'a> {
        match self {
            NamespaceChild::Dto(dto) => &dto.attributes,
            NamespaceChild::Rpc(rpc) => &rpc.attributes,
            NamespaceChild::Namespace(namespace) => &namespace.attributes,
        }
    }

    pub fn attributes_mut(&mut self) -> &mut Attributes<'a> {
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

impl<'a> TypeRef<'a> {
    pub fn new(fqtn: &[&'a str]) -> Self {
        Self {
            fully_qualified_type_name: fqtn.to_vec(),
        }
    }

    pub fn parent(&self) -> Option<Self> {
        let fqtn = &self.fully_qualified_type_name;
        if fqtn.is_empty() {
            return None;
        }
        let len = fqtn.len() - 1;
        Some(Self {
            fully_qualified_type_name: fqtn[..len].to_vec(),
        })
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

impl<'a, T> From<T> for TypeRef<'a>
where
    T: AsRef<[&'a str]>,
{
    fn from(value: T) -> Self {
        Self {
            fully_qualified_type_name: value.as_ref().to_vec(),
        }
    }
}

impl Display for TypeRef<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.fully_qualified_type_name.iter().join("."))
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

    /// Iterate over all [Attributes] of children in this [Namespace].
    pub fn child_attributes(&self) -> impl Iterator<Item = &Attributes<'a>> {
        self.children.iter().map(|child| child.attributes())
    }

    /// Iterate mutably over all [Attributes] of children in this [Namespace].
    pub fn child_attributes_mut(&mut self) -> impl Iterator<Item = &mut Attributes<'a>> {
        self.children.iter_mut().map(|child| child.attributes_mut())
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

    pub fn apply_attr_to_children_recursively(&mut self, key: &'a str, value: &str) {
        for attr in self.child_attributes_mut() {
            attr.insert(key, value.to_string());
        }
        for namespace in self.namespaces_mut() {
            namespace.apply_attr_to_children_recursively(key, value);
        }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::model::{Api, Dto, Namespace, Rpc};
    use crate::parser::Parser;
    use crate::{input, parser};

    #[test]
    fn merge() {
        let mut input0 = input::Buffer::new(
            r#"
            fn rpc0() {}
            struct dto0 {}
            mod nested0 {}
        "#,
        );
        let mut ns0 = test_api(&mut input0);

        let mut input1 = input::Buffer::new(
            r#"
            fn rpc1() {}
            struct dto1 {}
            mod nested1 {}
        "#,
        );
        let ns1 = test_api(&mut input1);

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

    mod parent {
        use crate::model::TypeRef;

        #[test]
        fn no_parent() {
            let ty = TypeRef::from([]);
            assert_eq!(ty.parent(), None);
        }

        #[test]
        fn parent_is_root() {
            let ty = TypeRef::from(["dto"]);
            assert_eq!(ty.parent(), Some([].into()));
        }

        #[test]
        fn typical() {
            let ty = TypeRef::from(["ns0", "ns1", "dto"]);
            let parent = ty.parent();
            assert_eq!(parent, Some(["ns0", "ns1"].into()));
            assert_eq!(parent.unwrap().parent(), Some(["ns0"].into()));
        }
    }

    #[test]
    fn apply_attr_to_children() {
        let mut input = input::Buffer::new(
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
        let mut api = test_api(&mut input);
        let attr_key = "key";
        let attr_value = "value".to_string();
        api.find_namespace_mut(&["ns0"].into())
            .unwrap()
            .apply_attr_to_children_recursively(attr_key, &attr_value);
        assert_eq!(
            api.find_namespace(&["ns0", "ns1"].into())
                .unwrap()
                .attributes
                .get(attr_key),
            Some(&attr_value)
        );
        assert_eq!(
            api.find_dto(&["ns0", "dto"].into())
                .unwrap()
                .attributes
                .get(attr_key),
            Some(&attr_value)
        );
        assert_eq!(
            api.find_rpc(&["ns0", "rpc"].into())
                .unwrap()
                .attributes
                .get(attr_key),
            Some(&attr_value)
        );
        assert_eq!(
            api.find_dto(&["ns0", "ns1", "dto"].into())
                .unwrap()
                .attributes
                .get(attr_key),
            Some(&attr_value)
        );
        assert_eq!(
            api.find_rpc(&["ns0", "ns1", "rpc"].into())
                .unwrap()
                .attributes
                .get(attr_key),
            Some(&attr_value)
        );
    }

    const NAMES: &[&str] = &["name0", "name1", "name2", "name3", "name4", "name5"];

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

    pub fn test_api(input: &mut input::Buffer) -> Api {
        parser::Rust::default()
            .parse(input)
            .expect("test api definition failed to parse")
            .into_api()
    }
}
