mod builder;

pub use builder::Builder;

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
}

impl<'a> TypeRef<'a> {
    pub fn namespace(&self) -> Vec<&'a str> {
        let len = self.fully_qualified_type_name.len();
        self.fully_qualified_type_name[..len - 1].to_vec()
    }

    pub fn name(&self) -> Option<&'a str> {
        self.fully_qualified_type_name.last().copied()
    }
}

impl<'a> Namespace<'a> {
    /// Merge segments from other namespace into this namespace. The other namespace's name is ignored.
    /// Note that this will preserve duplicate segments.
    pub fn merge(&mut self, mut other: Namespace<'a>) {
        self.segments.append(&mut other.segments)
    }

    // todo macro segment iters

    pub fn dtos(&self) -> impl Iterator<Item = &Dto> {
        self.segments.iter().filter_map(|segment| {
            if let Segment::Dto(dto) = segment {
                Some(dto)
            } else {
                None
            }
        })
    }

    pub fn rpcs(&self) -> impl Iterator<Item = &Rpc> {
        self.segments.iter().filter_map(|segment| {
            if let Segment::Rpc(rpc) = segment {
                Some(rpc)
            } else {
                None
            }
        })
    }

    pub fn namespaces(&self) -> impl Iterator<Item = &Namespace> {
        self.segments.iter().filter_map(|segment| {
            if let Segment::Namespace(namespace) = segment {
                Some(namespace)
            } else {
                None
            }
        })
    }

    // todo macro segment find by type ref

    pub fn find_namespace_mut(&mut self, type_ref: &TypeRef) -> Option<&mut Namespace<'a>> {
        let mut it = self;
        for name in &type_ref.fully_qualified_type_name {
            let segment = it
                .segments
                .iter_mut()
                .find(|s| matches!(s, Segment::Namespace(ns) if ns.name == *name));
            if let Some(Segment::Namespace(namespace)) = segment {
                it = namespace;
            } else {
                return None;
            }
        }
        Some(it)
    }
}
