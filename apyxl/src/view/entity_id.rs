use crate::model;
use itertools::Itertools;
use std::borrow::Cow;
use std::fmt::Debug;

#[derive(Debug, Copy, Clone)]
pub struct EntityId<'v, 'a> {
    target: &'v model::EntityId<'a>,
    xforms: &'v Vec<Box<dyn EntityIdTransform>>,
}

pub trait EntityIdTransform: Debug {
    fn fully_qualified_type_name(&self, _: &mut Vec<Cow<str>>) {}
}

impl<'v, 'a> EntityId<'v, 'a> {
    pub fn new(
        target: &'v model::EntityId<'a>,
        xforms: &'v Vec<Box<dyn EntityIdTransform>>,
    ) -> Self {
        Self { target, xforms }
    }

    pub fn fully_qualified_type_name(&self) -> Vec<Cow<str>> {
        let mut value = self
            .target
            .path
            .iter()
            .map(|s| Cow::Borrowed(*s))
            .collect_vec();
        for x in self.xforms {
            x.fully_qualified_type_name(&mut value)
        }
        value
    }
}

#[cfg(test)]
mod tests {
    use crate::test_util::executor::TestExecutor;
    use crate::view::tests::TestRenamer;
    use itertools::Itertools;

    #[test]
    fn fully_qualified_type_name() {
        #[test]
        fn name() {
            let mut exe = TestExecutor::new(
                r#"
                    struct dto {
                        field: some::Type
                    }
                "#,
            );
            let model = exe.model();
            let view = model.view().with_dto_transform(TestRenamer {});
            let root = view.api();
            let dto = root.find_dto(&["dto"].into()).unwrap();
            let fields = dto.fields().collect_vec();
            let field_type_id = fields.get(0).unwrap().ty();

            assert_eq!(
                field_type_id
                    .fully_qualified_type_name()
                    .iter()
                    .map(|s| s.as_ref())
                    .collect_vec(),
                vec!["some", "Type", TestRenamer::SUFFIX],
            );
        }
    }
}
