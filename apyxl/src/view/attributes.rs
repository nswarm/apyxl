use crate::model;
use std::fmt::Debug;

#[derive(Debug, Copy, Clone)]
pub struct Attributes<'v, 'a> {
    target: &'v model::Attributes<'a>,
    xforms: &'v Vec<Box<dyn AttributeTransform>>,
}

impl<'v, 'a> Attributes<'v, 'a> {
    pub fn new(
        target: &'v model::Attributes<'a>,
        xforms: &'v Vec<Box<dyn AttributeTransform>>,
    ) -> Self {
        Self { target, xforms }
    }
}

pub trait AttributeTransform: Debug {
    // todo
}

#[cfg(test)]
mod tests {
    #[test]
    fn asdf() {
        todo!()
    }
}
