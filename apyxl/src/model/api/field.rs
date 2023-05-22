use crate::model::{Attributes, Type};

/// A pair of name and type that describe a named instance of a type e.g. within a [Dto] or [Rpc].
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Field<'a> {
    pub name: &'a str,
    pub ty: Type,
    pub attributes: Attributes,
}
