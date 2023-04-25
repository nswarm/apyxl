use std::borrow::Cow;
use std::fmt::Debug;

use dyn_clone::DynClone;

use crate::model;
use crate::view::{Attributes, Dto, Rpc, Transforms};

/// A named, nestable wrapper for a set of API entities.
/// Wraps [model::Namespace].
#[derive(Debug, Copy, Clone)]
pub struct Namespace<'v, 'a> {
    target: &'v model::Namespace<'a>,
    xforms: &'v Transforms,
}

/// Wraps [model::NamespaceChild].
#[derive(Debug, Copy, Clone)]
pub enum NamespaceChild<'v, 'a> {
    Dto(Dto<'v, 'a>),
    Rpc(Rpc<'v, 'a>),
    Namespace(Namespace<'v, 'a>),
}

pub trait NamespaceTransform: Debug + DynClone {
    fn name(&self, _: &mut Cow<str>) {}

    /// `true`: included.
    /// `false`: excluded.
    fn filter_namespace(&self, _: &model::Namespace) -> bool {
        true
    }

    /// `true`: included.
    /// `false`: excluded.
    fn filter_dto(&self, _: &model::Dto) -> bool {
        true
    }

    /// `true`: included.
    /// `false`: excluded.
    fn filter_rpc(&self, _: &model::Rpc) -> bool {
        true
    }
}

dyn_clone::clone_trait_object!(NamespaceTransform);

impl<'v, 'a> NamespaceChild<'v, 'a> {
    pub fn new(target: &'v model::NamespaceChild<'a>, xforms: &'v Transforms) -> Self {
        match target {
            model::NamespaceChild::Dto(target) => NamespaceChild::Dto(Dto::new(target, &xforms)),
            model::NamespaceChild::Namespace(target) => {
                NamespaceChild::Namespace(Namespace::new(target, &xforms))
            }
            model::NamespaceChild::Rpc(target) => NamespaceChild::Rpc(Rpc::new(target, &xforms)),
        }
    }
}

impl<'v, 'a> Namespace<'v, 'a> {
    pub fn new(target: &'v model::Namespace<'a>, xforms: &'v Transforms) -> Self {
        Self { target, xforms }
    }

    pub fn clone_with_new_transforms(&self, xforms: &'v Transforms) -> Self {
        Self {
            target: self.target,
            xforms,
        }
    }

    pub fn name(&self) -> Cow<str> {
        let mut name = self.target.name.clone();
        for x in &self.xforms.namespace {
            x.name(&mut name)
        }
        name
    }

    pub fn children(&'a self) -> impl Iterator<Item = NamespaceChild<'v, 'a>> + 'a {
        self.target
            .children
            .iter()
            .filter(|child| match child {
                model::NamespaceChild::Dto(value) => self.filter_dto(value),
                model::NamespaceChild::Rpc(value) => self.filter_rpc(value),
                model::NamespaceChild::Namespace(value) => self.filter_namespace(value),
            })
            .map(|child| NamespaceChild::new(child, self.xforms))
    }

    pub fn attributes(&self) -> Attributes {
        Attributes::new(&self.target.attributes, &self.xforms.attr_xforms)
    }

    pub fn find_namespace(&'a self, id: &model::EntityId<'a>) -> Option<Namespace<'v, 'a>> {
        self.target
            .find_namespace(id)
            .filter(|namespace| {
                self.xforms
                    .namespace
                    .iter()
                    .all(|x| x.filter_namespace(namespace))
            })
            .map(|namespace| Namespace::new(namespace, self.xforms))
    }

    pub fn find_dto(&'a self, id: &model::EntityId<'a>) -> Option<Dto<'v, 'a>> {
        self.target
            .find_dto(id)
            .filter(|dto| self.xforms.namespace.iter().all(|x| x.filter_dto(dto)))
            .map(|dto| Dto::new(dto, self.xforms))
    }

    pub fn find_rpc(&'a self, id: &model::EntityId<'a>) -> Option<Rpc<'v, 'a>> {
        self.target
            .find_rpc(id)
            .filter(|rpc| self.xforms.namespace.iter().all(|x| x.filter_rpc(rpc)))
            .map(|rpc| Rpc::new(rpc, self.xforms))
    }

    pub fn namespaces(&'a self) -> impl Iterator<Item = Namespace<'v, 'a>> + 'a {
        self.target
            .namespaces()
            .filter(|ns| self.filter_namespace(ns))
            .map(move |ns| Namespace::new(ns, self.xforms))
    }

    pub fn dtos(&'a self) -> impl Iterator<Item = Dto<'v, 'a>> {
        self.target
            .dtos()
            .filter(|dto| self.filter_dto(dto))
            .map(move |dto| Dto::new(dto, self.xforms))
    }

    pub fn rpcs(&'a self) -> impl Iterator<Item = Rpc<'v, 'a>> {
        self.target
            .rpcs()
            .filter(|rpc| self.filter_rpc(rpc))
            .map(move |rpc| Rpc::new(rpc, self.xforms))
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

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use crate::test_util::executor::TestExecutor;
    use crate::view::tests::{TestFilter, TestRenamer};
    use crate::view::{NamespaceChild, Transformer};

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

        let visible_id = ["ns0", "visible"].into();
        let expected = model.api.find_namespace(&visible_id);
        let found = root.find_namespace(&visible_id);
        assert_eq!(found.map(|v| v.target), expected);

        let hidden_id = ["ns0", "hidden"].into();
        let found = root.find_namespace(&hidden_id);
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

        let visible_id = ["ns0", "visible"].into();
        let expected = model.api.find_dto(&visible_id).unwrap();
        let found = root.find_dto(&visible_id).unwrap();
        assert_eq!(found.name(), expected.name);

        let hidden_id = ["ns0", "hidden"].into();
        let found = root.find_dto(&hidden_id);
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

        let visible_id = ["ns0", "visible"].into();
        let expected = model.api.find_rpc(&visible_id).unwrap();
        let found = root.find_rpc(&visible_id).unwrap();
        assert_eq!(found.name(), expected.name);

        let hidden_id = ["ns0", "hidden"].into();
        let found = root.find_rpc(&hidden_id);
        assert!(found.is_none());
    }

    #[test]
    fn children() {
        let mut exe = TestExecutor::new(
            r#"
                    mod visible {}
                    mod hidden {}
                    struct visible {}
                    struct hidden {}
                    fn visible() {}
                    fn hidden() {}
                "#,
        );
        let model = exe.model();
        let view = model.view().with_namespace_transform(TestFilter {});
        let root = view.api();

        let children = root
            .children()
            .map(|v| match v {
                NamespaceChild::Dto(value) => value.name().to_string(),
                NamespaceChild::Rpc(value) => value.name().to_string(),
                NamespaceChild::Namespace(value) => value.name().to_string(),
            })
            .collect_vec();
        assert_eq!(children, vec!["visible", "visible", "visible"]);
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

        let namespaces = root
            .namespaces()
            .map(|v| v.name().to_string())
            .collect_vec();
        assert_eq!(namespaces, vec!["visible0", "visible1"],);
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

        let dtos = root.dtos().map(|v| v.name().to_string()).collect_vec();
        assert_eq!(dtos, vec!["visible0", "visible1",]);
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

        let rpcs = root.rpcs().map(|v| v.name().to_string()).collect_vec();
        assert_eq!(rpcs, vec!["visible0", "visible1"]);
    }
}
