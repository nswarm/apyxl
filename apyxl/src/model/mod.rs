/// A complete set of components that make up an API.
pub type Api<'a> = Namespace<'a>;

/// When parsing, the root namespace should be given this name so that when generating it can be
/// special cased as necessary.
pub const ROOT_NAMESPACE: &str = "_";

#[derive(Debug)]
pub enum Segment<'a> {
    Dto(Dto<'a>),
    Rpc(Rpc<'a>),
    Namespace(Namespace<'a>),
}

/// A named, nestable wrapper for a set of API components.
#[derive(Default, Debug)]
pub struct Namespace<'a> {
    pub name: &'a str,
    pub segments: Vec<Segment<'a>>,
}

/// A single Data Transfer Object (DTO) used in an [Rpc], either directly or nested in another [Dto].
#[derive(Default, Debug)]
pub struct Dto<'a> {
    pub name: &'a str,
    pub fields: Vec<Field<'a>>,
}

/// A pair of name and type that describe a named instance of a type e.g. within a [Dto] or [Rpc].
#[derive(Default, Debug)]
pub struct Field<'a> {
    pub name: &'a str,
    pub ty: TypeRef<'a>,
}

/// A single Remote Procedure Call (RPC) within an [Api].
#[derive(Default, Debug)]
pub struct Rpc<'a> {
    pub name: &'a str,
    pub params: Vec<Field<'a>>,
    pub return_type: Option<TypeRef<'a>>,
}

/// A type such as a language primitive or a reference to a [Dto]. A primitive will likely only have
/// the [name] field. a [Dto] reference will contain all necessary information to find the exact [Dto]
/// within the API.
#[derive(Default, Debug)]
pub struct TypeRef<'a> {
    // todo namespace(s)
    pub name: &'a str,
}

impl Namespace<'_> {
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
}
