use std::borrow::Cow;

use crate::model;

// In everything below,
//   'v: view
//   'a: api

struct Model<'v, 'a> {
    model: &'v model::Model<'a>,
    xforms: Transforms,
}

#[derive(Default)]
struct Transforms {
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

    pub fn root(&'v self) -> Namespace<'v, 'a> {
        Namespace::<'v, 'a> {
            target: &self.model.api,
            xforms: &self.xforms,
        }
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

struct Namespace<'v, 'a> {
    target: &'v model::Namespace<'a>,
    xforms: &'v Transforms,
}
struct Dto<'v, 'a> {
    target: &'v model::Dto<'a>,
    xforms: &'v Transforms,
}
struct Rpc<'v, 'a> {
    target: &'v model::Rpc<'a>,
    xforms: &'v Transforms,
}

impl<'v, 'a> Namespace<'v, 'a> {
    pub fn name(&self) -> Cow<str> {
        let mut name = Cow::Borrowed(self.target.name);
        for x in &self.xforms.namespace {
            x.name(&mut name)
        }
        name
    }

    pub fn find_namespace(&'a self, type_ref: &model::TypeRef<'a>) -> Option<Namespace<'v, 'a>> {
        self.target
            .find_namespace(type_ref)
            .map(|namespace| self.view_namespace(namespace))
            .filter(|namespace| {
                self.xforms
                    .namespace
                    .iter()
                    .all(|x| !x.filter_namespace(namespace))
            })
    }

    pub fn find_dto(&'a self, type_ref: &model::TypeRef<'a>) -> Option<Dto<'v, 'a>> {
        self.target
            .find_dto(type_ref)
            .map(|dto| self.view_dto(dto))
            .filter(|dto| self.xforms.namespace.iter().all(|x| !x.filter_dto(dto)))
    }

    pub fn find_rpc(&'a self, type_ref: &model::TypeRef<'a>) -> Option<Rpc<'v, 'a>> {
        self.target
            .find_rpc(type_ref)
            .map(|rpc| self.view_rpc(rpc))
            .filter(|rpc| self.xforms.namespace.iter().all(|x| !x.filter_rpc(rpc)))
    }

    pub fn namespaces(&'a self) -> impl Iterator<Item = Namespace<'v, 'a>> + 'a {
        self.target
            .namespaces()
            .map(move |ns| self.view_namespace(ns))
            .filter(|ns| self.filter_namespace(ns))
    }

    pub fn dtos(&'a self) -> impl Iterator<Item = Dto<'v, 'a>> {
        self.target
            .dtos()
            .map(move |dto| self.view_dto(dto))
            .filter(|dto| self.filter_dto(dto))
    }

    pub fn rpcs(&'a self) -> impl Iterator<Item = Rpc<'v, 'a>> {
        self.target
            .rpcs()
            .map(move |rpc| self.view_rpc(rpc))
            .filter(|rpc| self.filter_rpc(rpc))
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

    fn filter_namespace(&self, namespace: &Namespace) -> bool {
        self.xforms
            .namespace
            .iter()
            .all(|x| !x.filter_namespace(namespace))
    }

    fn filter_dto(&self, dto: &Dto) -> bool {
        self.xforms.namespace.iter().all(|x| !x.filter_dto(dto))
    }

    fn filter_rpc(&self, rpc: &Rpc) -> bool {
        self.xforms.namespace.iter().all(|x| !x.filter_rpc(rpc))
    }
}

impl<'v, 'a> Dto<'v, 'a> {
    pub fn name(&self) -> Cow<str> {
        let mut name = Cow::Borrowed(self.target.name);
        for x in &self.xforms.dto {
            x.name(&mut name)
        }
        name
    }
}

impl<'v, 'a> Rpc<'v, 'a> {
    pub fn name(&self) -> Cow<str> {
        let mut name = Cow::Borrowed(self.target.name);
        for x in &self.xforms.rpc {
            x.name(&mut name)
        }
        name
    }
}

trait NamespaceTransform {
    fn name(&self, _: &mut Cow<str>) {}
    fn filter_namespace(&self, _: &Namespace) -> bool {
        false
    }
    fn filter_dto(&self, _: &Dto) -> bool {
        false
    }
    fn filter_rpc(&self, _: &Rpc) -> bool {
        false
    }
}
trait DtoTransform {
    fn name(&self, _: &mut Cow<str>) {}
}
trait RpcTransform {
    fn name(&self, _: &mut Cow<str>) {}
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use crate::model::tests::test_api;
    use crate::view::{Dto, DtoTransform, Namespace, NamespaceTransform, Rpc, RpcTransform};
    use crate::{input, model};

    mod namespace {
        use itertools::Itertools;

        use crate::input;
        use crate::view::tests::{test_model, TestFilter, TestRenamer};
        use crate::view::Model;

        #[test]
        fn name() {
            let mut input = input::Buffer::new(
                r#"
                    mod ns0 {
                        mod ns1 {}
                    }
                "#,
            );
            let model = test_model(&mut input);
            let view = Model::new(&model).with_namespace_transform(TestRenamer {});
            let root = view.root();

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
            let mut input = input::Buffer::new(
                r#"
                    mod ns0 {
                        mod visible {}
                        mod hidden {}
                    }
                "#,
            );
            let model = test_model(&mut input);
            let view = Model::new(&model).with_namespace_transform(TestFilter {});
            let root = view.root();

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
            let mut input = input::Buffer::new(
                r#"
                    mod ns0 {
                        struct visible {}
                        struct hidden {}
                    }
                "#,
            );
            let model = test_model(&mut input);
            let view = Model::new(&model).with_namespace_transform(TestFilter {});
            let root = view.root();

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
            let mut input = input::Buffer::new(
                r#"
                    mod ns0 {
                        fn visible() {}
                        fn hidden() {}
                    }
                "#,
            );
            let model = test_model(&mut input);
            let view = Model::new(&model).with_namespace_transform(TestFilter {});
            let root = view.root();

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
            let mut input = input::Buffer::new(
                r#"
                    mod ns0 {
                        mod visible0 {}
                        mod hidden {}
                        mod visible1 {}
                    }
                "#,
            );
            let model = test_model(&mut input);
            let view = Model::new(&model).with_namespace_transform(TestFilter {});
            let root = view.root();

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
            let mut input = input::Buffer::new(
                r#"
                    mod ns0 {
                        struct visible0 {}
                        struct hidden {}
                        struct visible1 {}
                    }
                "#,
            );
            let model = test_model(&mut input);
            let view = Model::new(&model).with_namespace_transform(TestFilter {});
            let root = view.root();

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
            let mut input = input::Buffer::new(
                r#"
                    mod ns0 {
                        fn visible0() {}
                        fn hidden() {}
                        fn visible1() {}
                    }
                "#,
            );
            let model = test_model(&mut input);
            let view = Model::new(&model).with_namespace_transform(TestFilter {});
            let root = view.root();

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
        use crate::input;
        use crate::view::tests::{test_model, TestRenamer};
        use crate::view::Model;

        #[test]
        fn name() {
            let mut input = input::Buffer::new(
                r#"
                    mod ns0 {
                        struct dto0 {}
                        mod ns1 {
                            struct dto1 {}
                        }
                    }
                "#,
            );
            let model = test_model(&mut input);
            let view = Model::new(&model).with_dto_transform(TestRenamer {});
            let root = view.root();

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
        use crate::input;
        use crate::view::tests::{test_model, TestRenamer};
        use crate::view::Model;

        #[test]
        fn name() {
            let mut input = input::Buffer::new(
                r#"
                    mod ns0 {
                        fn rpc0() {}
                        mod ns1 {
                            fn rpc1() {}
                        }
                    }
                "#,
            );
            let model = test_model(&mut input);
            let view = Model::new(&model).with_rpc_transform(TestRenamer {});
            let root = view.root();

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

    #[derive(Default)]
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

    struct TestFilter {}
    impl NamespaceTransform for TestFilter {
        fn filter_namespace(&self, namespace: &Namespace) -> bool {
            namespace.name().contains("hidden")
        }

        fn filter_dto(&self, dto: &Dto) -> bool {
            dto.name().contains("hidden")
        }

        fn filter_rpc(&self, rpc: &Rpc) -> bool {
            rpc.name().contains("hidden")
        }
    }

    fn test_model(input: &mut input::Buffer) -> model::Model {
        model::Model {
            api: test_api(input),
            ..Default::default()
        }
    }
}
