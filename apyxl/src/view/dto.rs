use crate::model;
use crate::model::entity::ToEntity;
use crate::model::EntityType;
use crate::view::{Attributes, Field, Namespace, Transforms};
use dyn_clone::DynClone;
use std::borrow::Cow;
use std::fmt::Debug;

/// A single Data Transfer Object (DTO) used in an [Rpc], either directly or nested in another [Dto].
/// Wraps [model::Dto].
#[derive(Debug, Copy, Clone)]
pub struct Dto<'v, 'a> {
    target: &'v model::Dto<'a>,
    xforms: &'v Transforms,
}

pub trait DtoTransform: Debug + DynClone {
    fn name(&self, _: &mut Cow<str>) {}

    /// `true`: included.
    /// `false`: excluded.
    fn filter_field(&self, _: &model::Field) -> bool {
        true
    }
}

dyn_clone::clone_trait_object!(DtoTransform);

impl<'v, 'a> Dto<'v, 'a> {
    pub fn new(target: &'v model::Dto<'a>, xforms: &'v Transforms) -> Self {
        Self { target, xforms }
    }

    pub fn name(&self) -> Cow<str> {
        let mut name = Cow::Borrowed(self.target.name);
        for x in &self.xforms.dto {
            x.name(&mut name)
        }
        name
    }

    pub fn entity_type(&self) -> EntityType {
        self.target.entity_type()
    }

    pub fn fields(&'a self) -> impl Iterator<Item = Field<'v, 'a>> {
        self.target
            .fields
            .iter()
            .filter(|field| self.filter_field(field))
            .map(move |field| {
                Field::new(
                    field,
                    &self.xforms.field,
                    &self.xforms.entity_id,
                    &self.xforms.attr,
                )
            })
    }

    pub fn attributes(&self) -> Attributes {
        Attributes::new(
            &self.target.attributes,
            &self.xforms.attr,
            &self.xforms.entity_id,
        )
    }

    pub fn namespace(&self) -> Option<Namespace<'v, 'a>> {
        self.target
            .namespace
            .as_ref()
            .map(|ns| Namespace::new(ns, self.xforms))
    }

    fn filter_field(&self, field: &model::Field) -> bool {
        self.xforms.dto.iter().all(|x| x.filter_field(field))
    }
}

#[cfg(test)]
mod tests {
    use crate::model::EntityId;
    use crate::test_util::executor::TestExecutor;
    use crate::view::tests::{TestFilter, TestRenamer};
    use crate::view::Transformer;
    use itertools::Itertools;

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
            root.find_dto(&EntityId::try_from("ns0.d:dto0").unwrap())
                .unwrap()
                .name(),
            TestRenamer::renamed("dto0")
        );
        assert_eq!(
            root.find_dto(&EntityId::try_from("ns0.ns1.d:dto1").unwrap())
                .unwrap()
                .name(),
            TestRenamer::renamed("dto1")
        );
    }

    #[test]
    fn fields() {
        let mut exe = TestExecutor::new(
            r#"
            struct dto {
                visible0: Type0,
                hidden: Type0,
                visible1: Type0,
            }
            "#,
        );
        let model = exe.model();
        let view = model.view().with_dto_transform(TestFilter {});
        let root = view.api();
        let dto = root
            .find_dto(&EntityId::try_from("d:dto").unwrap())
            .unwrap();
        let fields = dto.fields().map(|f| f.name().to_string()).collect_vec();

        assert_eq!(fields, vec!["visible0", "visible1"]);
    }
}
