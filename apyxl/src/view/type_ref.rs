use crate::model;
use itertools::Itertools;
use std::borrow::Cow;
use std::fmt::Debug;

#[derive(Debug, Copy, Clone)]
pub struct TypeRef<'v, 'a> {
    target: &'v model::TypeRef<'a>,
    xforms: &'v Vec<Box<dyn TypeRefTransform>>,
}

pub trait TypeRefTransform: Debug {
    fn fully_qualified_type_name(&self, _: &mut Vec<Cow<str>>) {}
}

impl<'v, 'a> TypeRef<'v, 'a> {
    pub fn new(target: &'v model::TypeRef<'a>, xforms: &'v Vec<Box<dyn TypeRefTransform>>) -> Self {
        Self { target, xforms }
    }

    pub fn fully_qualified_type_name(&self) -> Vec<Cow<str>> {
        let mut value = self
            .target
            .fully_qualified_type_name
            .iter()
            .map(|s| Cow::Borrowed(*s))
            .collect_vec();
        for x in self.xforms {
            x.fully_qualified_type_name(&mut value)
        }
        value
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn fully_qualified_type_name() {
        todo!()
    }
}
