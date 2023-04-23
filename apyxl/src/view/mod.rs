use std::fmt::Debug;

pub use attributes::*;
pub use dto::*;
pub use entity_id::*;
pub use field::*;
pub use namespace::*;
pub use rpc::*;
pub use sub_view::*;

use crate::model;

mod attributes;
mod dto;
mod entity_id;
mod field;
mod namespace;
mod rpc;
mod sub_view;

// In everything in this module and submodules:
//   'v: view
//   'a: api

#[derive(Debug)]
pub struct Model<'v, 'a> {
    model: &'v model::Model<'a>,
    xforms: Transforms,
}

#[derive(Debug, Default, Clone)]
pub struct Transforms {
    namespace: Vec<Box<dyn NamespaceTransform>>,
    dto: Vec<Box<dyn DtoTransform>>,
    dto_field: Vec<Box<dyn FieldTransform>>,
    rpc: Vec<Box<dyn RpcTransform>>,
    rpc_param: Vec<Box<dyn FieldTransform>>,
    entity_id_xforms: Vec<Box<dyn EntityIdTransform>>,
    attr_xforms: Vec<Box<dyn AttributeTransform>>,
}

impl<'v, 'a> Model<'v, 'a> {
    pub fn new(model: &'v model::Model<'a>) -> Self {
        Self {
            model,
            xforms: Transforms::default(),
        }
    }

    pub fn api(&'v self) -> Namespace<'v, 'a> {
        Namespace::new(&self.model.api, &self.xforms)
    }

    // todo view::Metadata + metadata xforms
    pub fn metadata(&self) -> &model::Metadata {
        &self.model.metadata
    }
}

impl Transformer for Model<'_, '_> {
    fn xforms(&mut self) -> &mut Transforms {
        &mut self.xforms
    }
}

pub(crate) trait Transformer: Sized {
    fn xforms(&mut self) -> &mut Transforms;

    fn with_namespace_transform(mut self, xform: impl NamespaceTransform + 'static) -> Self {
        self.xforms().namespace.push(Box::new(xform));
        self
    }

    fn with_dto_transform(mut self, xform: impl DtoTransform + 'static) -> Self {
        self.xforms().dto.push(Box::new(xform));
        self
    }

    fn with_rpc_transform(mut self, xform: impl RpcTransform + 'static) -> Self {
        self.xforms().rpc.push(Box::new(xform));
        self
    }

    fn with_field_transform(mut self, xform: impl FieldTransform + 'static) -> Self {
        self.xforms().dto_field.push(Box::new(xform));
        self
    }

    fn with_entity_id_transform(mut self, xform: impl EntityIdTransform + 'static) -> Self {
        self.xforms().entity_id_xforms.push(Box::new(xform));
        self
    }
}

impl Transforms {
    pub fn namespace(&self) -> impl Iterator<Item = &Box<dyn NamespaceTransform>> {
        self.namespace.iter()
    }
    pub fn dto(&self) -> impl Iterator<Item = &Box<dyn DtoTransform>> {
        self.dto.iter()
    }
    pub fn dto_field(&self) -> impl Iterator<Item = &Box<dyn FieldTransform>> {
        self.dto_field.iter()
    }
    pub fn rpc(&self) -> impl Iterator<Item = &Box<dyn RpcTransform>> {
        self.rpc.iter()
    }
    pub fn rpc_param(&self) -> impl Iterator<Item = &Box<dyn FieldTransform>> {
        self.rpc_param.iter()
    }
    pub fn entity_id_xforms(&self) -> impl Iterator<Item = &Box<dyn EntityIdTransform>> {
        self.entity_id_xforms.iter()
    }
    pub fn attr_xforms(&self) -> impl Iterator<Item = &Box<dyn AttributeTransform>> {
        self.attr_xforms.iter()
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use crate::model;
    use crate::view::{
        DtoTransform, EntityIdTransform, FieldTransform, NamespaceTransform, RpcTransform,
    };

    #[derive(Default, Debug, Clone)]
    pub struct TestRenamer {}
    impl TestRenamer {
        pub const SUFFIX: &'static str = "renamed";

        pub fn renamed(name: &str) -> String {
            format!("{}_{}", name, TestRenamer::SUFFIX)
        }
    }

    impl NamespaceTransform for TestRenamer {
        fn name(&self, value: &mut Cow<str>) {
            *value = Cow::Owned(TestRenamer::renamed(value))
        }
    }
    impl DtoTransform for TestRenamer {
        fn name(&self, value: &mut Cow<str>) {
            *value = Cow::Owned(TestRenamer::renamed(value))
        }
    }
    impl RpcTransform for TestRenamer {
        fn name(&self, value: &mut Cow<str>) {
            *value = Cow::Owned(TestRenamer::renamed(value))
        }
    }
    impl FieldTransform for TestRenamer {
        fn name(&self, value: &mut Cow<str>) {
            *value = Cow::Owned(TestRenamer::renamed(value))
        }
    }
    impl EntityIdTransform for TestRenamer {
        fn path(&self, value: &mut Vec<Cow<str>>) {
            value.push(Cow::Borrowed(TestRenamer::SUFFIX))
        }
    }

    #[derive(Debug, Clone)]
    pub struct TestFilter {}
    impl NamespaceTransform for TestFilter {
        fn filter_namespace(&self, namespace: &model::Namespace) -> bool {
            !namespace.name.contains("hidden")
        }

        fn filter_dto(&self, dto: &model::Dto) -> bool {
            !dto.name.contains("hidden")
        }
        fn filter_rpc(&self, rpc: &model::Rpc) -> bool {
            !rpc.name.contains("hidden")
        }
    }

    impl DtoTransform for TestFilter {
        fn filter_field(&self, field: &model::Field) -> bool {
            !field.name.contains("hidden")
        }
    }

    impl RpcTransform for TestFilter {
        fn filter_param(&self, param: &model::Field) -> bool {
            !param.name.contains("hidden")
        }
    }
}
