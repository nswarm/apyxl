use std::borrow::Cow;
use std::fmt::Debug;

use dyn_clone::DynClone;

use crate::model;
use crate::view::{Attributes, Transforms};

/// A single enum within an [Api].
/// Wraps [model::Enum].
#[derive(Debug, Copy, Clone)]
pub struct Enum<'v, 'a> {
    target: &'v model::Enum<'a>,
    xforms: &'v Transforms,
}

/// A single enum within an [Api].
/// Wraps [model::Enum].
#[derive(Debug, Copy, Clone)]
pub struct EnumValue<'v, 'a> {
    target: &'v model::EnumValue<'a>,
    xforms: &'v Vec<Box<dyn EnumValueTransform>>,
}

pub trait EnumTransform: Debug + DynClone {
    fn name(&self, _: &mut Cow<str>) {}

    /// `true`: included.
    /// `false`: excluded.
    fn filter_value(&self, _: &model::EnumValue) -> bool {
        true
    }
}
dyn_clone::clone_trait_object!(EnumTransform);

pub trait EnumValueTransform: Debug + DynClone {
    fn name(&self, _: &mut Cow<str>) {}
    fn number(&self, _: &mut model::EnumValueNumber) {}
}
dyn_clone::clone_trait_object!(EnumValueTransform);

impl<'v, 'a> Enum<'v, 'a> {
    pub fn new(target: &'v model::Enum<'a>, xforms: &'v Transforms) -> Self {
        Self { target, xforms }
    }

    pub fn name(&self) -> Cow<str> {
        let mut name = Cow::Borrowed(self.target.name);
        for x in &self.xforms.en {
            x.name(&mut name)
        }
        name
    }

    pub fn values(&'a self) -> impl Iterator<Item = EnumValue<'v, 'a>> {
        self.target
            .values
            .iter()
            .filter(|value| self.filter_value(value))
            .map(move |value| EnumValue::new(value, &self.xforms.en_value))
    }

    pub fn attributes(&self) -> Attributes {
        Attributes::new(&self.target.attributes, &self.xforms.attr)
    }

    fn filter_value(&self, value: &model::EnumValue) -> bool {
        self.xforms.en.iter().all(|x| x.filter_value(value))
    }
}

impl<'v, 'a> EnumValue<'v, 'a> {
    pub fn new(
        target: &'v model::EnumValue<'a>,
        xforms: &'v Vec<Box<dyn EnumValueTransform>>,
    ) -> Self {
        Self { target, xforms }
    }

    pub fn name(&self) -> Cow<str> {
        let mut name = Cow::Borrowed(self.target.name);
        for x in self.xforms {
            x.name(&mut name)
        }
        name
    }

    pub fn number(&self) -> model::EnumValueNumber {
        let mut number = self.target.number;
        for x in self.xforms {
            x.number(&mut number)
        }
        number
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
                        fn en0() {}
                        mod ns1 {
                            fn en1() {}
                        }
                    }
                "#,
        );
        let model = exe.model();
        let view = model.view().with_enum_transform(TestRenamer {});
        let root = view.api();

        assert_eq!(
            root.find_enum(&EntityId::from("ns0.en0")).unwrap().name(),
            TestRenamer::renamed("en0")
        );
        assert_eq!(
            root.find_enum(&EntityId::from("ns0.ns1.en1"))
                .unwrap()
                .name(),
            TestRenamer::renamed("en1")
        );
    }

    #[test]
    fn values() {
        let mut exe = TestExecutor::new(
            r#"
            enum en {
                visible0 = 0,
                hidden0 = 1,
                visible1 = 2,
                hidden1 = 3,
            }
            "#,
        );
        let model = exe.model();
        let view = model.view().with_enum_transform(TestFilter {});
        let root = view.api();
        let en = root.find_enum(&EntityId::from("en")).unwrap();
        let values = en
            .values()
            .map(|value| value.name().to_string())
            .collect_vec();

        assert_eq!(values, vec!["visible0", "visible1"]);
    }
}
