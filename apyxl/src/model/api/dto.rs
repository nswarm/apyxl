use crate::model::{Attributes, Field};

/// A single Data Transfer Object (DTO) used in an [Rpc], either directly or nested in another [Dto].
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct Dto<'a> {
    pub name: &'a str,
    pub fields: Vec<Field<'a>>,
    pub attributes: Attributes,
}

impl<'a> Dto<'a> {
    pub fn field(&self, name: &str) -> Option<&Field<'a>> {
        self.fields.iter().find(|field| field.name == name)
    }

    pub fn field_mut(&mut self, name: &str) -> Option<&mut Field<'a>> {
        self.fields.iter_mut().find(|field| field.name == name)
    }
}
