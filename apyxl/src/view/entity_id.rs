use std::borrow::Cow;
use std::fmt::Debug;

use itertools::Itertools;

use crate::model;

/// A reference to another entity within the [Api].
#[derive(Debug, Copy, Clone)]
pub struct EntityId<'v, 'a> {
    target: &'v model::EntityId<'a>,
    xforms: &'v Vec<Box<dyn EntityIdTransform>>,
}

pub trait EntityIdTransform: Debug {
    fn path(&self, _: &mut Vec<Cow<str>>) {}
}

impl<'v, 'a> EntityId<'v, 'a> {
    pub fn new(
        target: &'v model::EntityId<'a>,
        xforms: &'v Vec<Box<dyn EntityIdTransform>>,
    ) -> Self {
        Self { target, xforms }
    }

    /// The path through other entities in the [Api] to get to the referred to entity. This will
    /// typically be a path through the hierarchy of [NamespaceChild], but can also refer to
    /// sub-child items like [Dto] fields or [Rpc] parameters.
    ///
    /// Examples:
    ///     `namespace1.namespace2.DtoName`
    ///     `namespace1.namespace2.DtoName.field0`
    ///     `namespace1.RpcName.param0`
    pub fn path(&self) -> Vec<Cow<str>> {
        let mut value = self
            .target
            .path
            .iter()
            .map(|s| Cow::Borrowed(*s))
            .collect_vec();
        for x in self.xforms {
            x.path(&mut value)
        }
        value
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use crate::test_util::executor::TestExecutor;
    use crate::view::tests::TestRenamer;

    #[test]
    fn path() {
        let mut exe = TestExecutor::new(
            r#"
                struct dto {
                    field: some::Type
                }
            "#,
        );
        let model = exe.model();
        let view = model.view().with_entity_id_transform(TestRenamer {});
        let root = view.api();
        let dto = root.find_dto(&["dto"].into()).unwrap();
        let fields = dto.fields().collect_vec();
        let field_type_id = fields.get(0).unwrap().ty();

        assert_eq!(
            field_type_id
                .path()
                .iter()
                .map(|s| s.as_ref())
                .collect_vec(),
            vec!["some", "Type", TestRenamer::SUFFIX],
        );
    }
}
