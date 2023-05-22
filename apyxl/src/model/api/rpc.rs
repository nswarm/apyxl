use crate::model::{Attributes, Field, Type};

/// A single Remote Procedure Call (RPC) within an [Api].
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct Rpc<'a> {
    pub name: &'a str,
    pub params: Vec<Field<'a>>,
    pub return_type: Option<Type>,
    pub attributes: Attributes,
}

impl<'a> Rpc<'a> {
    pub fn param(&self, name: &str) -> Option<&Field<'a>> {
        self.params.iter().find(|param| param.name == name)
    }

    pub fn param_mut(&mut self, name: &str) -> Option<&mut Field<'a>> {
        self.params.iter_mut().find(|param| param.name == name)
    }
}
