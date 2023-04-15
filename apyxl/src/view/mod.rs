use std::borrow::Cow;
use std::fmt::Debug;

use itertools::Itertools;

use crate::model;

// In everything below,
//   'v: view
//   'a: api

#[derive(Debug)]
pub struct Model<'v, 'a> {
    model: &'v model::Model<'a>,
    xforms: Transforms,
}

#[derive(Default, Debug)]
pub struct Transforms {
    namespace: Vec<Box<dyn NamespaceTransform>>,
    dto: Vec<Box<dyn DtoTransform>>,
    rpc: Vec<Box<dyn RpcTransform>>,
    // dto_field: Vec<Box<dyn FieldTransform>>,
    // rpc_param: Vec<Box<dyn FieldTransform>>,
    // rpc_return_type: Vec<Box<dyn FieldTransform>>, // TypeRefTransform?
}

impl<'v, 'a> Model<'v, 'a> {
    pub fn new(model: &'v model::Model<'a>) -> Self {
        Self {
            model,
            xforms: Transforms::default(),
        }
    }

    pub fn api(&'v self) -> Namespace<'v, 'a> {
        Namespace::<'v, 'a> {
            target: &self.model.api,
            xforms: &self.xforms,
        }
    }

    // todo view::Metadata
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
}

#[derive(Debug, Copy, Clone)]
pub struct TypeRef<'v, 'a> {
    target: &'v model::TypeRef<'a>,
    xforms: &'v Transforms,
}
#[derive(Debug, Copy, Clone)]
pub struct Namespace<'v, 'a> {
    target: &'v model::Namespace<'a>,
    xforms: &'v Transforms,
}
#[derive(Debug, Copy, Clone)]
pub struct NamespaceChild<'v, 'a> {
    target: &'v model::NamespaceChild<'a>,
    xforms: &'v Transforms,
}
#[derive(Debug, Copy, Clone)]
pub struct Dto<'v, 'a> {
    target: &'v model::Dto<'a>,
    xforms: &'v Transforms,
}
#[derive(Debug, Copy, Clone)]
pub struct Rpc<'v, 'a> {
    target: &'v model::Rpc<'a>,
    xforms: &'v Transforms,
}
#[derive(Debug, Copy, Clone)]
pub struct Field<'v, 'a> {
    target: &'v model::Field<'a>,
    xforms: &'v Transforms,
}
#[derive(Debug, Copy, Clone)]
pub struct Attributes<'v, 'a> {
    target: &'v model::Attributes<'a>,
    xforms: &'v Transforms,
}

// Helper for tests that wraps a view around an existing model type.
macro_rules! for_test {
    ($ty:ident) => {
        #[cfg(test)]
        pub fn for_test(xforms: &'v Transforms, target: &'v model::$ty<'a>) -> Self {
            Self {
                target,
                xforms: xforms,
            }
        }
    };
}

impl<'v, 'a> TypeRef<'v, 'a> {
    for_test!(TypeRef);

    pub fn fully_qualified_type_name(&self) -> Vec<Cow<str>> {
        self.target
            .fully_qualified_type_name
            .iter()
            .map(|s| Cow::Borrowed(*s))
            .collect_vec()
        // todo xforms
        // for x in &self.xforms.namespace {
        //     x.name(&mut name)
        // }
    }
}

impl<'v, 'a> Namespace<'v, 'a> {
    for_test!(Namespace);

    pub fn name(&self) -> Cow<str> {
        let mut name = Cow::Borrowed(self.target.name);
        for x in &self.xforms.namespace {
            x.name(&mut name)
        }
        name
    }

    pub fn children(&'a self) -> impl Iterator<Item = NamespaceChild<'v, 'a>> + 'a {
        // todo xforms
        self.target
            .children
            .iter()
            .map(|child| self.view_namespace_child(child))
    }

    pub fn attributes(&self) -> Attributes {
        // todo xforms
        self.view_attributes(&self.target.attributes)
    }

    pub fn find_namespace(&'a self, type_ref: &model::TypeRef<'a>) -> Option<Namespace<'v, 'a>> {
        self.target
            .find_namespace(type_ref)
            .filter(|namespace| {
                self.xforms
                    .namespace
                    .iter()
                    .all(|x| x.filter_namespace(namespace))
            })
            .map(|namespace| self.view_namespace(namespace))
    }

    pub fn find_dto(&'a self, type_ref: &model::TypeRef<'a>) -> Option<Dto<'v, 'a>> {
        self.target
            .find_dto(type_ref)
            .filter(|dto| self.xforms.namespace.iter().all(|x| x.filter_dto(dto)))
            .map(|dto| self.view_dto(dto))
    }

    pub fn find_rpc(&'a self, type_ref: &model::TypeRef<'a>) -> Option<Rpc<'v, 'a>> {
        self.target
            .find_rpc(type_ref)
            .filter(|rpc| self.xforms.namespace.iter().all(|x| x.filter_rpc(rpc)))
            .map(|rpc| self.view_rpc(rpc))
    }

    pub fn namespaces(&'a self) -> impl Iterator<Item = Namespace<'v, 'a>> + 'a {
        self.target
            .namespaces()
            .filter(|ns| self.filter_namespace(ns))
            .map(move |ns| self.view_namespace(ns))
    }

    pub fn dtos(&'a self) -> impl Iterator<Item = Dto<'v, 'a>> {
        self.target
            .dtos()
            .filter(|dto| self.filter_dto(dto))
            .map(move |dto| self.view_dto(dto))
    }

    pub fn rpcs(&'a self) -> impl Iterator<Item = Rpc<'v, 'a>> {
        self.target
            .rpcs()
            .filter(|rpc| self.filter_rpc(rpc))
            .map(move |rpc| self.view_rpc(rpc))
    }

    fn view_namespace_child(&'a self, child: &'a model::NamespaceChild) -> NamespaceChild {
        NamespaceChild {
            target: child,
            xforms: self.xforms,
        }
    }

    fn view_namespace(&'a self, namespace: &'a model::Namespace) -> Namespace {
        Namespace {
            target: namespace,
            xforms: self.xforms,
        }
    }

    fn view_dto(&'a self, dto: &'a model::Dto) -> Dto {
        Dto {
            target: dto,
            xforms: self.xforms,
        }
    }

    fn view_rpc(&'a self, rpc: &'a model::Rpc) -> Rpc {
        Rpc {
            target: rpc,
            xforms: self.xforms,
        }
    }

    fn view_attributes(&'a self, attributes: &'a model::Attributes) -> Attributes {
        Attributes {
            target: attributes,
            xforms: self.xforms,
        }
    }

    fn filter_namespace(&self, namespace: &model::Namespace) -> bool {
        self.xforms
            .namespace
            .iter()
            .all(|x| x.filter_namespace(namespace))
    }

    fn filter_dto(&self, dto: &model::Dto) -> bool {
        self.xforms.namespace.iter().all(|x| x.filter_dto(dto))
    }

    fn filter_rpc(&self, rpc: &model::Rpc) -> bool {
        self.xforms.namespace.iter().all(|x| x.filter_rpc(rpc))
    }
}

impl<'v, 'a> Dto<'v, 'a> {
    for_test!(Dto);

    pub fn name(&self) -> Cow<str> {
        let mut name = Cow::Borrowed(self.target.name);
        for x in &self.xforms.dto {
            x.name(&mut name)
        }
        name
    }

    pub fn fields(&'a self) -> impl Iterator<Item = Field<'v, 'a>> {
        self.target
            .fields
            .iter()
            .filter(|field| self.filter_field(field))
            .map(move |field| self.view_field(field))
    }

    fn filter_field(&self, field: &model::Field) -> bool {
        self.xforms.dto.iter().all(|x| x.filter_field(field))
    }

    fn view_field(&'a self, field: &'a model::Field) -> Field {
        Field {
            target: field,
            xforms: self.xforms,
        }
    }
}

impl<'v, 'a> Rpc<'v, 'a> {
    for_test!(Rpc);

    pub fn name(&self) -> Cow<str> {
        let mut name = Cow::Borrowed(self.target.name);
        for x in &self.xforms.rpc {
            x.name(&mut name)
        }
        name
    }

    pub fn params(&'a self) -> impl Iterator<Item = Field<'v, 'a>> {
        self.target
            .params
            .iter()
            .filter(|param| self.filter_param(param))
            .map(move |param| self.view_param(param))
    }

    pub fn return_type(&self) -> Option<TypeRef> {
        self.target
            .return_type
            .as_ref()
            .map(|type_ref| self.view_type_ref(type_ref))
        // todo xforms
        // for x in &self.xforms.rpc {
        //     x.name(&mut name)
        // }
    }

    fn filter_param(&self, param: &model::Field) -> bool {
        // todo xforms
        // self.xforms.dto.iter().all(|x| x.filter_param(param))
        return true;
    }

    fn view_param(&'a self, param: &'a model::Field) -> Field {
        Field {
            target: param,
            xforms: self.xforms,
        }
    }

    fn view_type_ref(&'a self, type_ref: &'a model::TypeRef) -> TypeRef {
        TypeRef {
            target: type_ref,
            xforms: self.xforms,
        }
    }
}

impl<'v, 'a> Field<'v, 'a> {
    for_test!(Field);

    pub fn name(&self) -> Cow<str> {
        let mut name = Cow::Borrowed(self.target.name);
        // todo xforms
        // for x in &self.xforms.field {
        //     x.name(&mut name)
        // }
        name
    }

    pub fn ty(&self) -> TypeRef {
        self.view_type_ref(&self.target.ty)
        // todo xforms
        // let mut ty = Cow::Borrowed(self.target.ty);
        // for x in &self.xforms.rpc {
        //     x.ty(&mut ty)
        // }
        // ty
    }

    fn view_type_ref(&'a self, type_ref: &'a model::TypeRef) -> TypeRef {
        TypeRef {
            target: type_ref,
            xforms: self.xforms,
        }
    }
}

pub trait NamespaceTransform: Debug {
    fn name(&self, _: &mut Cow<str>) {}

    /// If this returns false, the value will be excluded.
    fn filter_namespace(&self, _: &model::Namespace) -> bool {
        true
    }

    /// If this returns false, the value will be excluded.
    fn filter_dto(&self, _: &model::Dto) -> bool {
        true
    }

    /// If this returns false, the value will be excluded.
    fn filter_rpc(&self, _: &model::Rpc) -> bool {
        true
    }
}
pub trait DtoTransform: Debug {
    fn name(&self, _: &mut Cow<str>) {}

    /// If this returns false, the value will be excluded.
    fn filter_field(&self, _: &model::Field) -> bool {
        true
    }
}
pub trait RpcTransform: Debug {
    fn name(&self, _: &mut Cow<str>) {}
    fn return_type(&self, _: &mut model::TypeRef) {}

    /// If this returns false, the value will be excluded.
    fn filter_params(&self, _: &model::Field) -> bool {
        true
    }
}

#[cfg(test)]
pub mod tests {
    use std::borrow::Cow;

    use crate::model;
    use crate::view::{DtoTransform, NamespaceTransform, RpcTransform};

    mod namespace {
        use itertools::Itertools;

        use crate::test_util::executor::TestExecutor;
        use crate::view::tests::{TestFilter, TestRenamer};

        #[test]
        fn name() {
            let mut exe = TestExecutor::new(
                r#"
                    mod ns0 {
                        mod ns1 {}
                    }
                "#,
            );
            let model = exe.model();
            let view = model.view();
            let view = model.view().with_namespace_transform(TestRenamer {});
            let root = view.api();

            assert_eq!(root.name(), TestRenamer::renamed("_"));
            assert_eq!(
                root.find_namespace(&["ns0"].into()).unwrap().name(),
                TestRenamer::renamed("ns0")
            );
            assert_eq!(
                root.find_namespace(&["ns0", "ns1"].into()).unwrap().name(),
                TestRenamer::renamed("ns1")
            );
        }

        #[test]
        fn find_namespace() {
            let mut exe = TestExecutor::new(
                r#"
                    mod ns0 {
                        mod visible {}
                        mod hidden {}
                    }
                "#,
            );
            let model = exe.model();
            let view = model.view().with_namespace_transform(TestFilter {});
            let root = view.api();

            let visible_type_ref = ["ns0", "visible"].into();
            let expected = model.api.find_namespace(&visible_type_ref);
            let found = root.find_namespace(&visible_type_ref);
            assert_eq!(found.map(|v| v.target), expected);

            let hidden_type_ref = ["ns0", "hidden"].into();
            let found = root.find_namespace(&hidden_type_ref);
            assert!(found.is_none());
        }

        #[test]
        fn find_dto() {
            let mut exe = TestExecutor::new(
                r#"
                    mod ns0 {
                        struct visible {}
                        struct hidden {}
                    }
                "#,
            );
            let model = exe.model();
            let view = model.view().with_namespace_transform(TestFilter {});
            let root = view.api();

            let visible_type_ref = ["ns0", "visible"].into();
            let expected = model.api.find_dto(&visible_type_ref);
            let found = root.find_dto(&visible_type_ref);
            assert_eq!(found.map(|v| v.target), expected);

            let hidden_type_ref = ["ns0", "hidden"].into();
            let found = root.find_dto(&hidden_type_ref);
            assert!(found.is_none());
        }

        #[test]
        fn find_rpc() {
            let mut exe = TestExecutor::new(
                r#"
                    mod ns0 {
                        fn visible() {}
                        fn hidden() {}
                    }
                "#,
            );
            let model = exe.model();
            let view = model.view().with_namespace_transform(TestFilter {});
            let root = view.api();

            let visible_type_ref = ["ns0", "visible"].into();
            let expected = model.api.find_rpc(&visible_type_ref);
            let found = root.find_rpc(&visible_type_ref);
            assert_eq!(found.map(|v| v.target), expected);

            let hidden_type_ref = ["ns0", "hidden"].into();
            let found = root.find_rpc(&hidden_type_ref);
            assert!(found.is_none());
        }

        #[test]
        fn namespaces() {
            let mut exe = TestExecutor::new(
                r#"
                    mod visible0 {}
                    mod hidden {}
                    mod visible1 {}
                "#,
            );
            let model = exe.model();
            let view = model.view().with_namespace_transform(TestFilter {});
            let root = view.api();

            let namespaces = root.namespaces().map(|v| v.target).collect_vec();
            assert_eq!(
                namespaces,
                vec![
                    model.api.find_namespace(&["visible0"].into()).unwrap(),
                    model.api.find_namespace(&["visible1"].into()).unwrap(),
                ],
            );
        }

        #[test]
        fn dtos() {
            let mut exe = TestExecutor::new(
                r#"
                    struct visible0 {}
                    struct hidden {}
                    struct visible1 {}
                "#,
            );
            let model = exe.model();
            let view = model.view().with_namespace_transform(TestFilter {});
            let root = view.api();

            let dtos = root.dtos().map(|v| v.target).collect_vec();
            assert_eq!(
                dtos,
                vec![
                    model.api.find_dto(&["visible0"].into()).unwrap(),
                    model.api.find_dto(&["visible1"].into()).unwrap(),
                ],
            );
        }

        #[test]
        fn rpcs() {
            let mut exe = TestExecutor::new(
                r#"
                    fn visible0() {}
                    fn hidden() {}
                    fn visible1() {}
                "#,
            );
            let model = exe.model();
            let view = model.view().with_namespace_transform(TestFilter {});
            let root = view.api();

            let rpcs = root.rpcs().map(|v| v.target).collect_vec();
            assert_eq!(
                rpcs,
                vec![
                    model.api.find_rpc(&["visible0"].into()).unwrap(),
                    model.api.find_rpc(&["visible1"].into()).unwrap(),
                ],
            );
        }
    }

    mod dto {
        use crate::test_util::executor::TestExecutor;
        use crate::view::tests::TestRenamer;

        #[test]
        fn name() {
            let mut exe = TestExecutor::new(
                r#"
                    mod ns0 {
                        struct dto0 {}
                        mod ns1 {
                            struct dto1 {}
                        }
                    }
                "#,
            );
            let model = exe.model();
            let view = model.view().with_dto_transform(TestRenamer {});
            let root = view.api();

            assert_eq!(
                root.find_dto(&["ns0", "dto0"].into()).unwrap().name(),
                TestRenamer::renamed("dto0")
            );
            assert_eq!(
                root.find_dto(&["ns0", "ns1", "dto1"].into())
                    .unwrap()
                    .name(),
                TestRenamer::renamed("dto1")
            );
        }
    }

    mod rpc {
        use crate::test_util::executor::TestExecutor;
        use crate::view::tests::TestRenamer;

        #[test]
        fn name() {
            let mut exe = TestExecutor::new(
                r#"
                    mod ns0 {
                        fn rpc0() {}
                        mod ns1 {
                            fn rpc1() {}
                        }
                    }
                "#,
            );
            let model = exe.model();
            let view = model.view().with_rpc_transform(TestRenamer {});
            let root = view.api();

            assert_eq!(
                root.find_rpc(&["ns0", "rpc0"].into()).unwrap().name(),
                TestRenamer::renamed("rpc0")
            );
            assert_eq!(
                root.find_rpc(&["ns0", "ns1", "rpc1"].into())
                    .unwrap()
                    .name(),
                TestRenamer::renamed("rpc1")
            );
        }
    }

    #[derive(Default, Debug)]
    struct TestRenamer {}
    impl TestRenamer {
        const SUFFIX: &'static str = "_renamed";

        fn renamed(name: &str) -> String {
            format!("{}{}", name, TestRenamer::SUFFIX)
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

    #[derive(Debug)]
    struct TestFilter {}
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
}
