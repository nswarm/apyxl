use std::borrow::Cow;

use crate::model::{Api, Dto, Namespace, Rpc, TypeRef};

// In all of the below:
//      'v: view
//      'a: api

struct ApiView<'v, 'a> {
    api: &'v Api<'a>,
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

impl<'v, 'a> ApiView<'v, 'a> {
    pub fn new(api: &'v Api<'a>) -> Self {
        Self {
            api,
            xforms: Transforms::default(),
        }
    }

    pub fn root(&'v self) -> NamespaceView<'v, 'a> {
        NamespaceView::<'v, 'a> {
            target: self.api,
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

struct NamespaceView<'v, 'a> {
    target: &'v Namespace<'a>,
    xforms: &'v Transforms,
}
struct DtoView<'v, 'a> {
    target: &'v Dto<'a>,
    xforms: &'v Transforms,
}
struct RpcView<'v, 'a> {
    target: &'v Rpc<'a>,
    xforms: &'v Transforms,
}

impl<'v, 'a> NamespaceView<'v, 'a> {
    pub fn name(&self) -> Cow<str> {
        let mut name = Cow::Borrowed(self.target.name);
        for x in &self.xforms.namespace {
            x.name(&mut name)
        }
        name
    }

    pub fn find_namespace(&'a self, type_ref: &TypeRef<'a>) -> Option<NamespaceView<'v, 'a>> {
        self.target
            .find_namespace(type_ref)
            .map(|namespace| self.namespace_to_view(namespace))
            .filter(|namespace| {
                self.xforms
                    .namespace
                    .iter()
                    .all(|x| !x.filter_namespace(namespace))
            })
    }

    pub fn find_dto(&'a self, type_ref: &TypeRef<'a>) -> Option<DtoView<'v, 'a>> {
        self.target
            .find_dto(type_ref)
            .map(|dto| self.dto_to_view(dto))
            .filter(|dto| self.xforms.namespace.iter().all(|x| !x.filter_dto(dto)))
    }

    pub fn find_rpc(&'a self, type_ref: &TypeRef<'a>) -> Option<RpcView<'v, 'a>> {
        self.target
            .find_rpc(type_ref)
            .map(|rpc| self.rpc_to_view(rpc))
            .filter(|rpc| self.xforms.namespace.iter().all(|x| !x.filter_rpc(rpc)))
    }

    pub fn namespaces(&'a self) -> impl Iterator<Item = NamespaceView<'v, 'a>> + 'a {
        self.target
            .namespaces()
            .map(move |ns| self.namespace_to_view(ns))
            .filter(|ns| self.filter_namespace(ns))
    }

    pub fn dtos(&'a self) -> impl Iterator<Item = DtoView<'v, 'a>> {
        self.target
            .dtos()
            .map(move |dto| self.dto_to_view(dto))
            .filter(|dto| self.filter_dto(dto))
    }

    pub fn rpcs(&'a self) -> impl Iterator<Item = RpcView<'v, 'a>> {
        self.target
            .rpcs()
            .map(move |rpc| self.rpc_to_view(rpc))
            .filter(|rpc| self.filter_rpc(rpc))
    }

    fn namespace_to_view(&'a self, namespace: &'a Namespace) -> NamespaceView {
        NamespaceView {
            target: namespace,
            xforms: self.xforms,
        }
    }

    fn dto_to_view(&'a self, dto: &'a Dto) -> DtoView {
        DtoView {
            target: dto,
            xforms: self.xforms,
        }
    }

    fn rpc_to_view(&'a self, rpc: &'a Rpc) -> RpcView {
        RpcView {
            target: rpc,
            xforms: self.xforms,
        }
    }

    fn filter_namespace(&self, namespace: &NamespaceView) -> bool {
        self.xforms
            .namespace
            .iter()
            .all(|x| !x.filter_namespace(namespace))
    }

    fn filter_dto(&self, dto: &DtoView) -> bool {
        self.xforms.namespace.iter().all(|x| !x.filter_dto(dto))
    }

    fn filter_rpc(&self, rpc: &RpcView) -> bool {
        self.xforms.namespace.iter().all(|x| !x.filter_rpc(rpc))
    }
}

impl<'v, 'a> DtoView<'v, 'a> {
    pub fn name(&self) -> Cow<str> {
        let mut name = Cow::Borrowed(self.target.name);
        for x in &self.xforms.dto {
            x.name(&mut name)
        }
        name
    }
}

impl<'v, 'a> RpcView<'v, 'a> {
    pub fn name(&self) -> Cow<str> {
        let mut name = Cow::Borrowed(self.target.name);
        for x in &self.xforms.rpc {
            x.name(&mut name)
        }
        name
    }
}

trait NamespaceTransform {
    fn name(&self, value: &mut Cow<str>) {}
    fn filter_namespace(&self, namespace: &NamespaceView) -> bool {
        false
    }
    fn filter_dto(&self, dto: &DtoView) -> bool {
        false
    }
    fn filter_rpc(&self, rpc: &RpcView) -> bool {
        false
    }
}
trait DtoTransform {
    fn name(&self, value: &mut Cow<str>) {}
}
trait RpcTransform {
    fn name(&self, value: &mut Cow<str>) {}
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use crate::model::view::{DtoView, NamespaceTransform, NamespaceView, RpcView};

    mod namespace {
        use itertools::Itertools;

        use crate::input;
        use crate::model::tests::test_api;
        use crate::model::view::tests::{TestFilter, TestRenamer};
        use crate::model::view::ApiView;

        #[test]
        fn name() {
            let mut input = input::Buffer::new(
                r#"
                    mod ns0 {
                        mod visible {}
                        mod hidden {}
                    }
                "#,
            );
            let api = test_api(&mut input);
            let view = ApiView::new(&api).with_namespace_transform(TestRenamer {});
            let root = view.root();

            assert_eq!(root.name(), TestRenamer::NAME);
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
            let api = test_api(&mut input);
            let view = ApiView::new(&api).with_namespace_transform(TestFilter {});
            let root = view.root();

            let visible_type_ref = ["ns0", "visible"].into();
            let expected = api.find_namespace(&visible_type_ref);
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
            let api = test_api(&mut input);
            let view = ApiView::new(&api).with_namespace_transform(TestFilter {});
            let root = view.root();

            let visible_type_ref = ["ns0", "visible"].into();
            let expected = api.find_dto(&visible_type_ref);
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
            let api = test_api(&mut input);
            let view = ApiView::new(&api).with_namespace_transform(TestFilter {});
            let root = view.root();

            let visible_type_ref = ["ns0", "visible"].into();
            let expected = api.find_rpc(&visible_type_ref);
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
            let api = test_api(&mut input);
            let view = ApiView::new(&api).with_namespace_transform(TestFilter {});
            let root = view.root();

            let namespaces = root.namespaces().map(|v| v.target).collect_vec();
            assert_eq!(
                namespaces,
                vec![
                    api.find_namespace(&["visible0"].into()).unwrap(),
                    api.find_namespace(&["visible1"].into()).unwrap(),
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
            let api = test_api(&mut input);
            let view = ApiView::new(&api).with_namespace_transform(TestFilter {});
            let root = view.root();

            let dtos = root.dtos().map(|v| v.target).collect_vec();
            assert_eq!(
                dtos,
                vec![
                    api.find_dto(&["visible0"].into()).unwrap(),
                    api.find_dto(&["visible1"].into()).unwrap(),
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
            let api = test_api(&mut input);
            let view = ApiView::new(&api).with_namespace_transform(TestFilter {});
            let root = view.root();

            let rpcs = root.rpcs().map(|v| v.target).collect_vec();
            assert_eq!(
                rpcs,
                vec![
                    api.find_rpc(&["visible0"].into()).unwrap(),
                    api.find_rpc(&["visible1"].into()).unwrap(),
                ],
            );
        }
    }

    mod dto {
        #[test]
        fn asdf() {
            todo!("nyi")
        }
    }

    mod rpc {
        #[test]
        fn asdf() {
            todo!("nyi")
        }
    }

    struct TestRenamer {}
    impl TestRenamer {
        const NAME: &'static str = "renamed";
    }
    impl NamespaceTransform for TestRenamer {
        fn name(&self, value: &mut Cow<str>) {
            *value = Cow::Borrowed(TestRenamer::NAME)
        }
    }

    struct TestFilter {}
    impl NamespaceTransform for TestFilter {
        fn filter_namespace(&self, namespace: &NamespaceView) -> bool {
            namespace.name().contains("hidden")
        }

        fn filter_dto(&self, dto: &DtoView) -> bool {
            dto.name().contains("hidden")
        }

        fn filter_rpc(&self, rpc: &RpcView) -> bool {
            rpc.name().contains("hidden")
        }
    }
}
