/// A complete set of components that make up an API.
#[derive(Default, Debug)]
pub struct Api {
    pub segments: Vec<Segment>,
}

impl Api {
    pub fn dtos(&self) -> impl Iterator<Item = &Dto> {
        self.segments.iter().filter_map(|segment| {
            if let Segment::Dto(dto) = segment {
                Some(dto)
            } else {
                None
            }
        })
    }
}

#[derive(Debug)]
pub enum Segment {
    Dto(Dto),
    Rpc(Rpc),
}

/// A single Data Transfer Object (DTO) used in an RPC, either directly or nested in another DTO.
#[derive(Default, Debug)]
pub struct Dto {
    pub name: String, // todo can be &str  prob
    pub fields: Vec<Field>,
}

/// A field on a DTO.
#[derive(Default, Debug)]
pub struct Field {
    pub name: String,
    pub ty: DtoRef,
}

/// A single Remote Procedure Call (RPC) within an API.
#[derive(Default, Debug)]
pub struct Rpc {
    pub name: String,
    pub params: Vec<Field>,
    pub return_type: Option<DtoRef>,
}

/// A reference to a DTO. Contains all necessary information to find the exact DTO within the API.
#[derive(Default, Debug)]
pub struct DtoRef {
    // todo namespace(s)
    pub name: String,
}
