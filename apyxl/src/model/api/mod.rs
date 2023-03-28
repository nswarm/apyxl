pub use builder::Builder;

mod builder;

/// A complete set of components that make up an API.
pub type Api<'a> = Namespace<'a>;

/// When parsing, the root namespace should be given this name so that when generating it can be
/// special cased as necessary.
pub const UNDEFINED_NAMESPACE: &str = "_";

#[derive(Debug, Eq, PartialEq)]
pub enum Segment<'a> {
    Dto(Dto<'a>),
    Rpc(Rpc<'a>),
    Namespace(Namespace<'a>),
}

/// A named, nestable wrapper for a set of API components.
#[derive(Default, Debug, Eq, PartialEq)]
pub struct Namespace<'a> {
    pub name: &'a str,
    pub segments: Vec<Segment<'a>>,
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
#[derive(Default, Debug, Eq, PartialEq)]
pub struct TypeRef<'a> {
    pub fully_qualified_type_name: Vec<&'a str>,
}

impl<'a> TypeRef<'a> {
    pub fn new(fqtn: &[&'a str]) -> Self {
        Self {
            fully_qualified_type_name: fqtn.to_vec(),
        }
    }

    pub fn has_namespace(&self) -> bool {
        self.fully_qualified_type_name.len() > 1
    }

    /// Returns an iterator over the part of the path _before_ the name, which represents the
    /// namespace it is a part of.
    pub fn namespace<'b>(&'b self) -> impl Iterator<Item = &'a str> + 'b {
        let len = self.fully_qualified_type_name.len();
        self.fully_qualified_type_name.iter().copied().take(len - 1)
    }

    /// The name is always the last part of the type path.
    pub fn name(&self) -> Option<&'a str> {
        self.fully_qualified_type_name.last().copied()
    }
}

impl<'a, T: Iterator<Item = &'a str>> From<T> for TypeRef<'a> {
    fn from(value: T) -> Self {
        TypeRef {
            fully_qualified_type_name: value.collect::<Vec<_>>(),
        }
    }
}

macro_rules! get {
    ($self: ident, $name: ident, $segment: ident, $iter: ident) => {
        $self.segments.$iter().find_map(|s| match s {
            Segment::$segment(value) if value.name == $name => Some(value),
            _ => None,
        })
    };
}

macro_rules! iter {
    ($self: ident, $segment:ident, $iter: ident) => {
        $self.segments.$iter().filter_map(|segment| {
            if let Segment::$segment(value) = segment {
                Some(value)
            } else {
                None
            }
        })
    };
}

macro_rules! find {
    ($self: ident, $type_ref: ident, $get: ident, $find_namespace: ident) => {{
        if !$type_ref.has_namespace() {
            return $type_ref.name().map_or(None, |name| $self.$get(name));
        }
        let namespace = $self.$find_namespace(&TypeRef::from($type_ref.namespace()));
        let name = $type_ref.name();
        match (namespace, name) {
            (Some(namespace), Some(name)) => namespace.$get(name),
            _ => None,
        }
    }};
}

impl<'a> Namespace<'a> {
    /// Merge segments from other namespace into this namespace. The other namespace's name is ignored.
    /// Note that this will preserve duplicate segments.
    pub fn merge(&mut self, mut other: Namespace<'a>) {
        self.segments.append(&mut other.segments)
    }

    /// Add a [Dto] as a segment in this [Namespace].
    pub fn add_dto(&mut self, value: Dto<'a>) {
        self.segments.push(Segment::Dto(value));
    }

    /// Add an [Rpc] as a segment in this [Namespace].
    pub fn add_rpc(&mut self, value: Rpc<'a>) {
        self.segments.push(Segment::Rpc(value));
    }

    /// Add a nested [Namespace] under this [Namespace].
    pub fn add_namespace(&mut self, value: Namespace<'a>) {
        self.segments.push(Segment::Namespace(value));
    }

    /// Get a [Dto] within this [Namespace] by name."
    fn dto(&self, name: &str) -> Option<&Dto<'a>> {
        get!(self, name, Dto, iter)
    }

    /// Get a [Dto] within this [Namespace] by name."
    fn dto_mut(&mut self, name: &str) -> Option<&mut Dto<'a>> {
        get!(self, name, Dto, iter_mut)
    }

    /// Get a [Rpc] within this [Namespace] by name."
    fn rpc(&self, name: &str) -> Option<&Rpc<'a>> {
        get!(self, name, Rpc, iter)
    }

    /// Get a [Rpc] within this [Namespace] by name."
    fn rpc_mut(&mut self, name: &str) -> Option<&mut Rpc<'a>> {
        get!(self, name, Rpc, iter_mut)
    }

    /// Get a [Namespace] within this [Namespace] by name."
    fn namespace(&self, name: &str) -> Option<&Namespace<'a>> {
        get!(self, name, Namespace, iter)
    }

    /// Get a [Namespace] within this [Namespace] by name."
    fn namespace_mut(&mut self, name: &str) -> Option<&mut Namespace<'a>> {
        get!(self, name, Namespace, iter_mut)
    }

    /// Iterate over all [Dto]s within this [Namespace].
    pub fn dtos(&self) -> impl Iterator<Item = &Dto<'a>> {
        iter!(self, Dto, iter)
    }

    /// Iterate over all [Rpc]s within this [Namespace].
    pub fn rpcs(&self) -> impl Iterator<Item = &Rpc<'a>> {
        iter!(self, Rpc, iter)
    }

    /// Iterate over all [Namespace]s within this [Namespace].
    pub fn namespaces(&self) -> impl Iterator<Item = &Namespace<'a>> {
        iter!(self, Namespace, iter)
    }

    /// Find a [Dto] by `type_ref` relative to this [Namespace].
    pub fn find_dto(&self, type_ref: &TypeRef) -> Option<&Dto<'a>> {
        find!(self, type_ref, dto, find_namespace)
    }

    /// Find a [Dto] by `type_ref` relative to this [Namespace].
    pub fn find_dto_mut(&mut self, type_ref: &TypeRef) -> Option<&mut Dto<'a>> {
        find!(self, type_ref, dto_mut, find_namespace_mut)
    }

    /// Find a [Rpc] by `type_ref` relative to this [Namespace].
    pub fn find_rpc(&self, type_ref: &TypeRef) -> Option<&Rpc<'a>> {
        find!(self, type_ref, rpc, find_namespace)
    }

    /// Find a [Rpc] by `type_ref` relative to this [Namespace].
    pub fn find_rpc_mut(&mut self, type_ref: &TypeRef) -> Option<&mut Rpc<'a>> {
        find!(self, type_ref, rpc_mut, find_namespace_mut)
    }

    /// Find a [Namespace] by `type_ref` relative to this [Namespace].
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
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{Api, Dto, Namespace, Rpc};

    mod add_get {
        use crate::model::api::tests::{
            complex_api, complex_namespace, test_dto, test_namespace, test_rpc, NAMES,
        };

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
            let api = complex_api();
            let type_ref1 = TypeRef::new(&[test_dto(1).name]);
            let type_ref2 = TypeRef::new(&[test_dto(2).name]);
            assert_eq!(api.find_dto(&type_ref1), Some(&test_dto(1)));
            assert_eq!(api.find_dto(&type_ref2), Some(&test_dto(2)));
        }

        #[test]
        fn rpc() {
            let api = complex_api();
            let type_ref1 = TypeRef::new(&[test_dto(1).name]);
            let type_ref2 = TypeRef::new(&[test_dto(2).name]);
            assert_eq!(api.find_rpc(&type_ref1), Some(&test_rpc(1)),);
            assert_eq!(api.find_rpc(&type_ref2), Some(&test_rpc(2)),);
        }

        #[test]
        fn namespace() {
            let api = complex_api();
            let type_ref1 = TypeRef::new(&[complex_namespace(1).name]);
            let type_ref2 = TypeRef::new(&[complex_namespace(2).name]);
            assert_eq!(api.find_namespace(&type_ref1), Some(&complex_namespace(1)));
            assert_eq!(api.find_namespace(&type_ref2), Some(&complex_namespace(2)));
        }

        #[test]
        fn child() {
            let api = complex_api();
            let type_ref = TypeRef::new(&[complex_namespace(1).name, NAMES[3]]);
            assert_eq!(api.find_dto(&type_ref), Some(&test_dto(3)));
            assert_eq!(api.find_rpc(&type_ref), Some(&test_rpc(3)));
            assert_eq!(api.find_namespace(&type_ref), Some(&test_namespace(3)));
        }
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
        namespace.add_namespace(test_namespace(i + 3));
        namespace
    }

    fn test_dto(i: usize) -> Dto<'static> {
        Dto {
            name: NAMES[i],
            ..Default::default()
        }
    }

    fn test_rpc(i: usize) -> Rpc<'static> {
        Rpc {
            name: NAMES[i],
            ..Default::default()
        }
    }

    fn test_namespace(i: usize) -> Namespace<'static> {
        Namespace {
            name: NAMES[i],
            ..Default::default()
        }
    }
}
