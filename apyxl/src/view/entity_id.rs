use dyn_clone::DynClone;
use itertools::Itertools;
use std::borrow::Cow;
use std::fmt::Debug;

use crate::model;

/// A reference to another entity within the [Api].
#[derive(Debug, Copy, Clone)]
pub struct EntityId<'v> {
    target: &'v model::EntityId,
    xforms: &'v Vec<Box<dyn EntityIdTransform>>,
}

pub trait EntityIdTransform: Debug + DynClone {
    fn path(&self, _: &mut Vec<Cow<str>>) {}
}

dyn_clone::clone_trait_object!(EntityIdTransform);

impl<'v> EntityId<'v> {
    pub fn new(target: &'v model::EntityId, xforms: &'v Vec<Box<dyn EntityIdTransform>>) -> Self {
        Self { target, xforms }
    }

    // The raw underlying EntityId. Only use this if trying to find something in the actual
    // model API.
    pub fn target(&self) -> &model::EntityId {
        self.target
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
            .component_names()
            .map(Cow::Borrowed)
            .collect_vec();
        for x in self.xforms {
            x.path(&mut value)
        }
        value
    }
}

#[cfg(test)]
mod tests {
    use crate::model::EntityId;
    use itertools::Itertools;

    use crate::test_util::executor::TestExecutor;
    use crate::view::tests::TestRenamer;
    use crate::view::Transformer;

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
        let dto = root
            .find_dto(&EntityId::try_from("dto:dto").unwrap())
            .unwrap();
        let fields = dto.fields().collect_vec();
        let ty = fields.first().unwrap().ty();

        assert_eq!(
            ty.value()
                .api()
                .unwrap()
                .path()
                .iter()
                .map(|s| s.as_ref())
                .collect_vec(),
            vec!["some", "Type", TestRenamer::SUFFIX],
        );
    }
}
