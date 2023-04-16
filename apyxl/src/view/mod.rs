use std::fmt::Debug;

pub use attributes::*;
pub use dto::*;
pub use field::*;
pub use namespace::*;
pub use rpc::*;
pub use type_ref::*;

use crate::model;

mod attributes;
mod dto;
mod field;
mod namespace;
mod rpc;
mod type_ref;

// In everything in this module and submodules:
//   'v: view
//   'a: api

#[derive(Debug)]
pub struct Model<'v, 'a> {
    model: &'v model::Model<'a>,
    xforms: Transforms,
}

#[derive(Default, Debug)]
pub struct Transforms {
    pub namespace: Vec<Box<dyn NamespaceTransform>>,
    pub dto: Vec<Box<dyn DtoTransform>>,
    pub dto_field: Vec<Box<dyn FieldTransform>>,
    pub rpc: Vec<Box<dyn RpcTransform>>,
    pub rpc_param: Vec<Box<dyn FieldTransform>>,
    pub type_ref_xforms: Vec<Box<dyn TypeRefTransform>>,
    pub attr_xforms: Vec<Box<dyn AttributeTransform>>,
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

    pub fn with_namespace_transform(mut self, xform: impl NamespaceTransform + 'static) -> Self {
        self.xforms.namespace.push(Box::new(xform));
        self
    }

    pub fn with_dto_transform(mut self, xform: impl DtoTransform + 'static) -> Self {
        self.xforms.dto.push(Box::new(xform));
        self
    }

    pub fn with_rpc_transform(mut self, xform: impl RpcTransform + 'static) -> Self {
        self.xforms.rpc.push(Box::new(xform));
        self
    }

    pub fn with_field_transform(mut self, xform: impl FieldTransform + 'static) -> Self {
        self.xforms.dto_field.push(Box::new(xform));
        self
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use crate::model;
    use crate::view::{DtoTransform, FieldTransform, NamespaceTransform, RpcTransform, TypeRef};

    #[derive(Default, Debug)]
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
        fn ty(&self, value: &mut TypeRef) {
            value
                .fully_qualified_type_name()
                .push(Cow::Borrowed(TestRenamer::SUFFIX))
        }
    }

    #[derive(Debug)]
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
