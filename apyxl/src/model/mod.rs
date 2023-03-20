/// A complete set of components that make up an API.
#[derive(Default, Debug)]
pub struct Api<'a> {
    pub segments: Vec<Segment<'a>>,
}

impl<'a> Api<'_> {
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
}

#[derive(Debug)]
pub enum Segment<'a> {
    Dto(Dto<'a>),
    Rpc(Rpc<'a>),
}

/// A single Data Transfer Object (DTO) used in an RPC, either directly or nested in another DTO.
#[derive(Default, Debug)]
pub struct Dto<'a> {
    pub name: &'a str,
    pub fields: Vec<Field<'a>>,
}

/// A field on a DTO.
#[derive(Default, Debug)]
pub struct Field<'a> {
    pub name: &'a str,
    pub ty: DtoRef<'a>,
}

/// A single Remote Procedure Call (RPC) within an API.
#[derive(Default, Debug)]
pub struct Rpc<'a> {
    pub name: &'a str,
    pub params: Vec<Field<'a>>,
    pub return_type: Option<DtoRef<'a>>,
}

/// A reference to a DTO. Contains all necessary information to find the exact DTO within the API.
#[derive(Default, Debug)]
pub struct DtoRef<'a> {
    // todo namespace(s)
    pub name: &'a str,
}
