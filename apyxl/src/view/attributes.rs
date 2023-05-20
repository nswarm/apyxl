use crate::model;
use dyn_clone::DynClone;
use std::fmt::Debug;
use crate::model::chunk;

#[derive(Debug, Copy, Clone)]
pub struct Attributes<'v> {
    target: &'v model::Attributes,
    xforms: &'v Vec<Box<dyn AttributeTransform>>,
}

impl<'v> Attributes<'v> {
    pub fn new(
        target: &'v model::Attributes,
        xforms: &'v Vec<Box<dyn AttributeTransform>>,
    ) -> Self {
        Self { target, xforms }
    }

    // todo transform
    pub fn chunk(&self) -> Option<&chunk::Attribute> {
        self.target.chunk.as_ref()
    }
}

pub trait AttributeTransform: Debug + DynClone {
    // todo
}

dyn_clone::clone_trait_object!(AttributeTransform);

// #[cfg(test)]
// mod tests {
//     #[test]
//     fn asdf() {
//         todo!()
//     }
// }
