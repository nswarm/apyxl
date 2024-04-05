use std::fmt::Debug;

use anyhow::anyhow;
use anyhow::Result;
use log::debug;

pub use attributes::*;
pub use dto::*;
pub use en::*;
pub use entity_id::*;
pub use field::*;
pub use namespace::*;
pub use rpc::*;
pub use sub_view::*;
pub use ty::*;
pub use ty_alias::*;

use crate::model;
use crate::model::Chunk;
use crate::model::chunk::ChunkFilter;

mod attributes;
mod dto;
mod en;
mod entity_id;
mod field;
mod namespace;
mod rpc;
mod sub_view;
mod ty;
mod ty_alias;

// In everything in this module and submodules:
//   'v: view
//   'a: api

/// An immutable view into the API [model::Model] with a set of [Transforms] that modify the data
/// read from the model.
#[derive(Debug, Clone)]
pub struct Model<'v, 'a> {
    target: &'v model::Model<'a>,
    xforms: Transforms,
}

#[derive(Debug, Default, Clone)]
pub struct Transforms {
    namespace: Vec<Box<dyn NamespaceTransform>>,
    dto: Vec<Box<dyn DtoTransform>>,
    dto_field: Vec<Box<dyn FieldTransform>>,
    rpc: Vec<Box<dyn RpcTransform>>,
    rpc_param: Vec<Box<dyn FieldTransform>>,
    en: Vec<Box<dyn EnumTransform>>,
    en_value: Vec<Box<dyn EnumValueTransform>>,
    ty_alias: Vec<Box<dyn TypeAliasTransform>>,
    entity_id: Vec<Box<dyn EntityIdTransform>>,
    attr: Vec<Box<dyn AttributeTransform>>,
}

impl<'v: 'a, 'a> Model<'v, 'a> {
    pub fn new(target: &'v model::Model<'a>) -> Self {
        Self {
            target,
            xforms: Transforms::default(),
        }
    }

    /// Get the full combined API root with all transforms applied.
    pub fn api(&'v self) -> Namespace<'v, 'a> {
        Namespace::new(&self.target.api(), &self.xforms)
    }

    /// Iterate over [Chunk]s, where each subsection of the API can be viewed through a [SubView]
    /// with all transforms, as well as a [ChunkFilter] for the appropriate chunk applied.
    pub fn api_chunked_iter(&self) -> impl Iterator<Item = Result<(&Chunk, SubView<'a>)>> {
        self.metadata().chunks.iter().map(|metadata| {
            let namespace = match self.target.api().find_namespace(&metadata.root_namespace) {
                None => {
                    return Err(anyhow!(
                        "could not find root namespace with id '{}' for chunk with path '{:?}'",
                        metadata.root_namespace,
                        metadata.chunk
                    ))
                }
                Some(namespace) => namespace,
            };
            let path = metadata.chunk.relative_file_path.as_ref().ok_or(anyhow!(
                "all chunks must have a relative_file_path when using chunked API"
            ))?;
            debug!(
                "writing chunk: path '{}' namespace '{}'",
                path.display(),
                metadata.root_namespace
            );
            let sub_view = SubView::new(
                metadata.root_namespace.clone(),
                namespace,
                self.xforms.clone(),
            )
            .with_namespace_transform(ChunkFilter::new(path));
            Ok((&metadata.chunk, sub_view))
        })
    }

    // todo view::Metadata + metadata xforms
    pub fn metadata(&self) -> &model::Metadata {
        &self.target.metadata()
    }

    pub fn dependencies(&self) -> &model::Dependencies {
        &self.target.dependencies()
    }
}

impl Transformer for Model<'_, '_> {
    fn xforms(&mut self) -> &mut Transforms {
        &mut self.xforms
    }
}

#[allow(dead_code)]
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

    fn with_enum_transform(mut self, xform: impl EnumTransform + 'static) -> Self {
        self.xforms().en.push(Box::new(xform));
        self
    }

    fn with_enum_value_transform(mut self, xform: impl EnumValueTransform + 'static) -> Self {
        self.xforms().en_value.push(Box::new(xform));
        self
    }

    fn with_field_transform(mut self, xform: impl FieldTransform + 'static) -> Self {
        self.xforms().dto_field.push(Box::new(xform));
        self
    }

    fn with_ty_alias_transform(mut self, xform: impl TypeAliasTransform + 'static) -> Self {
        self.xforms().ty_alias.push(Box::new(xform));
        self
    }

    fn with_entity_id_transform(mut self, xform: impl EntityIdTransform + 'static) -> Self {
        self.xforms().entity_id.push(Box::new(xform));
        self
    }

    fn with_attribute_transform(mut self, xform: impl AttributeTransform + 'static) -> Self {
        self.xforms().attr.push(Box::new(xform));
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
    pub fn ty_alias(&self) -> impl Iterator<Item = &Box<dyn TypeAliasTransform>> {
        self.ty_alias.iter()
    }
    pub fn entity_id_xforms(&self) -> impl Iterator<Item = &Box<dyn EntityIdTransform>> {
        self.entity_id.iter()
    }
    pub fn attr_xforms(&self) -> impl Iterator<Item = &Box<dyn AttributeTransform>> {
        self.attr.iter()
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use crate::model;
    use crate::view::{
        DtoTransform, EntityIdTransform, EnumTransform, EnumValueTransform,
        FieldTransform, NamespaceTransform, RpcTransform,
    };
    use crate::view::ty_alias::TypeAliasTransform;

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
    impl EnumTransform for TestRenamer {
        fn name(&self, value: &mut Cow<str>) {
            *value = Cow::Owned(TestRenamer::renamed(value))
        }
    }
    impl EnumValueTransform for TestRenamer {
        fn name(&self, value: &mut Cow<str>) {
            *value = Cow::Owned(TestRenamer::renamed(value))
        }
    }
    impl FieldTransform for TestRenamer {
        fn name(&self, value: &mut Cow<str>) {
            *value = Cow::Owned(TestRenamer::renamed(value))
        }
    }
    impl TypeAliasTransform for TestRenamer {
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
        fn filter_enum(&self, en: &model::Enum) -> bool {
            !en.name.contains("hidden")
        }
        fn filter_ty_alias(&self, en: &model::TypeAlias) -> bool {
            !en.name.contains("hidden")
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

    impl EnumTransform for TestFilter {
        fn filter_value(&self, value: &model::EnumValue) -> bool {
            !value.name.contains("hidden")
        }
    }
}
