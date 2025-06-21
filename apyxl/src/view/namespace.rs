use std::borrow::Cow;
use std::fmt::Debug;

use dyn_clone::DynClone;

use crate::model;
use crate::model::entity::ToEntity;
use crate::model::EntityType;
use crate::view::ty_alias::TypeAlias;
use crate::view::{Attributes, Dto, Enum, Field, Rpc, Transforms};

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
    Enum(Enum<'v, 'a>),
    TypeAlias(TypeAlias<'v, 'a>),
    Field(Field<'v, 'a>),
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

    /// `true`: included.
    /// `false`: excluded.
    fn filter_enum(&self, _: &model::Enum) -> bool {
        true
    }

    /// `true`: included.
    /// `false`: excluded.
    fn filter_ty_alias(&self, _: &model::TypeAlias) -> bool {
        true
    }

    /// `true`: included.
    /// `false`: excluded.
    fn filter_field(&self, _: &model::Field) -> bool {
        true
    }
}

dyn_clone::clone_trait_object!(NamespaceTransform);

impl<'v, 'a> NamespaceChild<'v, 'a> {
    pub fn new(target: &'v model::NamespaceChild<'a>, xforms: &'v Transforms) -> Self {
        match target {
            model::NamespaceChild::Dto(target) => NamespaceChild::Dto(Dto::new(target, xforms)),
            model::NamespaceChild::Namespace(target) => {
                NamespaceChild::Namespace(Namespace::new(target, xforms))
            }
            model::NamespaceChild::Enum(target) => NamespaceChild::Enum(Enum::new(target, xforms)),
            model::NamespaceChild::Rpc(target) => NamespaceChild::Rpc(Rpc::new(target, xforms)),
            model::NamespaceChild::Field(target) => NamespaceChild::Field(Field::new(
                target,
                &xforms.field,
                &xforms.entity_id,
                &xforms.attr,
            )),
            model::NamespaceChild::TypeAlias(target) => {
                NamespaceChild::TypeAlias(TypeAlias::new(target, xforms))
            }
        }
    }

    pub fn name(&self) -> Cow<str> {
        match self {
            NamespaceChild::Dto(dto) => dto.name(),
            NamespaceChild::Rpc(rpc) => rpc.name(),
            NamespaceChild::Enum(en) => en.name(),
            NamespaceChild::TypeAlias(alias) => alias.name(),
            NamespaceChild::Field(field) => field.name(),
            NamespaceChild::Namespace(namespace) => namespace.name(),
        }
    }

    pub fn attributes(&self) -> Attributes {
        match self {
            NamespaceChild::Dto(dto) => dto.attributes(),
            NamespaceChild::Rpc(rpc) => rpc.attributes(),
            NamespaceChild::Enum(en) => en.attributes(),
            NamespaceChild::TypeAlias(alias) => alias.attributes(),
            NamespaceChild::Field(field) => field.attributes(),
            NamespaceChild::Namespace(namespace) => namespace.attributes(),
        }
    }

    pub fn entity_type(&self) -> EntityType {
        match self {
            NamespaceChild::Dto(dto) => dto.entity_type(),
            NamespaceChild::Rpc(rpc) => rpc.entity_type(),
            NamespaceChild::Enum(en) => en.entity_type(),
            NamespaceChild::TypeAlias(alias) => alias.entity_type(),
            NamespaceChild::Field(field) => field.entity_type(),
            NamespaceChild::Namespace(namespace) => namespace.entity_type(),
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

    pub fn entity_type(&self) -> EntityType {
        self.target.entity_type()
    }

    pub fn is_empty(&self) -> bool {
        self.children().count() == 0
    }

    pub fn children(&'a self) -> impl Iterator<Item = NamespaceChild<'v, 'a>> + 'a {
        self.target
            .children
            .iter()
            .filter(|child| self.filter_child(child))
            .map(|child| NamespaceChild::new(child, self.xforms))
    }

    pub fn attributes(&self) -> Attributes {
        Attributes::new(
            &self.target.attributes,
            &self.xforms.attr,
            &self.xforms.entity_id,
        )
    }

    pub fn find_child(&'a self, id: &model::EntityId) -> Option<NamespaceChild<'v, 'a>> {
        self.target
            .find_descendant(id)
            .filter(|child| self.filter_child(child))
            .map(|child| NamespaceChild::new(child, self.xforms))
    }

    pub fn find_namespace(&'a self, id: &model::EntityId) -> Option<Namespace<'v, 'a>> {
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

    pub fn find_dto(&'a self, id: &model::EntityId) -> Option<Dto<'v, 'a>> {
        self.target
            .find_dto(id)
            .filter(|dto| self.filter_dto(dto))
            .map(|dto| Dto::new(dto, self.xforms))
    }

    pub fn find_rpc(&'a self, id: &model::EntityId) -> Option<Rpc<'v, 'a>> {
        self.target
            .find_rpc(id)
            .filter(|rpc| self.filter_rpc(rpc))
            .map(|rpc| Rpc::new(rpc, self.xforms))
    }

    pub fn find_enum(&'a self, id: &model::EntityId) -> Option<Enum<'v, 'a>> {
        self.target
            .find_enum(id)
            .filter(|en| self.filter_enum(en))
            .map(|en| Enum::new(en, self.xforms))
    }

    pub fn find_ty_alias(&'a self, id: &model::EntityId) -> Option<TypeAlias<'v, 'a>> {
        self.target
            .find_ty_alias(id)
            .filter(|alias| self.filter_ty_alias(alias))
            .map(|en| TypeAlias::new(en, self.xforms))
    }

    pub fn namespaces(&'a self) -> impl Iterator<Item = Namespace<'v, 'a>> + 'a {
        self.target
            .namespaces()
            .filter(|ns| self.filter_namespace(ns))
            .map(|ns| Namespace::new(ns, self.xforms))
    }

    pub fn dtos(&'a self) -> impl Iterator<Item = Dto<'v, 'a>> {
        self.target
            .dtos()
            .filter(|dto| self.filter_dto(dto))
            .map(|dto| Dto::new(dto, self.xforms))
    }

    pub fn rpcs(&'a self) -> impl Iterator<Item = Rpc<'v, 'a>> {
        self.target
            .rpcs()
            .filter(|rpc| self.filter_rpc(rpc))
            .map(|rpc| Rpc::new(rpc, self.xforms))
    }

    pub fn enums(&'a self) -> impl Iterator<Item = Enum<'v, 'a>> {
        self.target
            .enums()
            .filter(|en| self.filter_enum(en))
            .map(|en| Enum::new(en, self.xforms))
    }

    pub fn ty_aliases(&'a self) -> impl Iterator<Item = TypeAlias<'v, 'a>> {
        self.target
            .ty_aliases()
            .filter(|alias| self.filter_ty_alias(alias))
            .map(|alias| TypeAlias::new(alias, self.xforms))
    }

    fn filter_child(&self, child: &model::NamespaceChild) -> bool {
        match child {
            model::NamespaceChild::Dto(value) => self.filter_dto(value),
            model::NamespaceChild::Rpc(value) => self.filter_rpc(value),
            model::NamespaceChild::Enum(value) => self.filter_enum(value),
            model::NamespaceChild::TypeAlias(value) => self.filter_ty_alias(value),
            model::NamespaceChild::Field(field) => self.filter_field(field),
            model::NamespaceChild::Namespace(value) => self.filter_namespace(value),
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

    fn filter_enum(&self, en: &model::Enum) -> bool {
        self.xforms.namespace.iter().all(|x| x.filter_enum(en))
    }

    fn filter_ty_alias(&self, alias: &model::TypeAlias) -> bool {
        self.xforms
            .namespace
            .iter()
            .all(|x| x.filter_ty_alias(alias))
    }

    fn filter_field(&self, field: &model::Field) -> bool {
        self.xforms.namespace.iter().all(|x| x.filter_field(field))
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use crate::model::EntityId;
    use crate::test_util::executor::TestExecutor;
    use crate::view::tests::{TestFilter, TestRenamer};
    use crate::view::Transformer;

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
            root.find_namespace(&EntityId::try_from("ns0").unwrap())
                .unwrap()
                .name(),
            TestRenamer::renamed("ns0")
        );
        assert_eq!(
            root.find_namespace(&EntityId::try_from("ns0.ns1").unwrap())
                .unwrap()
                .name(),
            TestRenamer::renamed("ns1")
        );
    }

    #[test]
    fn find_child() {
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

        let visible_id = EntityId::try_from("ns0.d:visible").unwrap();
        let expected = model.api().find_descendant(&visible_id).unwrap();
        let found = root.find_child(&visible_id).unwrap();
        assert_eq!(found.name(), expected.name());

        let hidden_id = EntityId::try_from("ns0.d:hidden").unwrap();
        let found = root.find_child(&hidden_id);
        assert!(found.is_none());
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

        let visible_id = EntityId::try_from("ns0.visible").unwrap();
        let expected = model.api().find_namespace(&visible_id);
        let found = root.find_namespace(&visible_id);
        assert_eq!(found.map(|v| v.target), expected);

        let hidden_id = EntityId::try_from("ns0.hidden").unwrap();
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

        let visible_id = EntityId::try_from("ns0.d:visible").unwrap();
        let expected = model.api().find_dto(&visible_id).unwrap();
        let found = root.find_dto(&visible_id).unwrap();
        assert_eq!(found.name(), expected.name);

        let hidden_id = EntityId::try_from("ns0.d:hidden").unwrap();
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

        let visible_id = EntityId::try_from("ns0.r:visible").unwrap();
        let expected = model.api().find_rpc(&visible_id).unwrap();
        let found = root.find_rpc(&visible_id).unwrap();
        assert_eq!(found.name(), expected.name);

        let hidden_id = EntityId::try_from("ns0.r:hidden").unwrap();
        let found = root.find_rpc(&hidden_id);
        assert!(found.is_none());
    }

    #[test]
    fn find_enum() {
        let mut exe = TestExecutor::new(
            r#"
                    mod ns0 {
                        enum visible {}
                        enum hidden {}
                    }
                "#,
        );
        let model = exe.model();
        let view = model.view().with_namespace_transform(TestFilter {});
        let root = view.api();

        let visible_id = EntityId::try_from("ns0.e:visible").unwrap();
        let expected = model.api().find_enum(&visible_id).unwrap();
        let found = root.find_enum(&visible_id).unwrap();
        assert_eq!(found.name(), expected.name);

        let hidden_id = EntityId::try_from("ns0.e:hidden").unwrap();
        let found = root.find_enum(&hidden_id);
        assert!(found.is_none());
    }

    #[test]
    fn find_alias() {
        let mut exe = TestExecutor::new(
            r#"
                    mod ns0 {
                        type visible = SomeType;
                        type hidden = SomeType;
                    }
                "#,
        );
        let model = exe.model();
        let view = model.view().with_namespace_transform(TestFilter {});
        let root = view.api();

        let visible_id = EntityId::try_from("ns0.alias:visible").unwrap();
        let expected = model.api().find_ty_alias(&visible_id).unwrap();
        let found = root.find_ty_alias(&visible_id).unwrap();
        assert_eq!(found.name(), expected.name);

        let hidden_id = EntityId::try_from("ns0.alias:hidden").unwrap();
        let found = root.find_ty_alias(&hidden_id);
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
                    enum visible {}
                    enum hidden {}
                    type visible = SomeType;
                    type hidden = SomeType;
                "#,
        );
        let model = exe.model();
        let view = model.view().with_namespace_transform(TestFilter {});
        let root = view.api();

        let children = root.children().map(|v| v.name().to_string()).collect_vec();
        assert_eq!(
            children,
            vec!["visible", "visible", "visible", "visible", "visible"]
        );
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

    #[test]
    fn enums() {
        let mut exe = TestExecutor::new(
            r#"
                    enum visible0 {}
                    enum hidden {}
                    enum visible1 {}
                "#,
        );
        let model = exe.model();
        let view = model.view().with_namespace_transform(TestFilter {});
        let root = view.api();

        let enums = root.enums().map(|v| v.name().to_string()).collect_vec();
        assert_eq!(enums, vec!["visible0", "visible1"]);
    }

    #[test]
    fn aliases() {
        let mut exe = TestExecutor::new(
            r#"
                    type visible0 = u32;
                    type hidden = u32;
                    type visible1 = u32;
                "#,
        );
        let model = exe.model();
        let view = model.view().with_namespace_transform(TestFilter {});
        let root = view.api();

        let aliases = root
            .ty_aliases()
            .map(|v| v.name().to_string())
            .collect_vec();
        assert_eq!(aliases, vec!["visible0", "visible1"]);
    }
}
