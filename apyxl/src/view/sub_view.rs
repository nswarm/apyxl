use crate::model;
use crate::view::{Namespace, Transformer, Transforms};

/// A view into the [Model] starting at a specific [Namespace] with additional [Transforms].
#[derive(Debug)]
pub struct SubView<'a> {
    namespace: &'a model::Namespace<'a>,
    xforms: Transforms,
}

impl<'a> SubView<'a> {
    pub fn new(namespace: &'a model::Namespace<'a>, xforms: Transforms) -> Self {
        Self { namespace, xforms }
    }

    pub fn namespace<'v>(&'v self) -> Namespace<'v, 'a> {
        Namespace::new(self.namespace, &self.xforms)
    }
}

impl Transformer for SubView<'_> {
    fn xforms(&mut self) -> &mut Transforms {
        &mut self.xforms
    }
}
