use std::borrow::Cow;
use std::fmt::Debug;

use dyn_clone::DynClone;

use crate::model;
use crate::model::entity::ToEntity;
use crate::model::EntityType;
use crate::view::{Attributes, Transforms, Type};

/// A single type alias within an [Api].
/// Wraps [model::TypeAlias].
#[derive(Debug, Copy, Clone)]
pub struct TypeAlias<'v, 'a> {
    target: &'v model::TypeAlias<'a>,
    xforms: &'v Transforms,
}

pub trait TypeAliasTransform: Debug + DynClone {
    fn name(&self, _: &mut Cow<str>) {}
}
dyn_clone::clone_trait_object!(TypeAliasTransform);

impl<'v, 'a> TypeAlias<'v, 'a> {
    pub fn new(target: &'v model::TypeAlias<'a>, xforms: &'v Transforms) -> Self {
        Self { target, xforms }
    }

    pub fn entity_type(&self) -> EntityType {
        self.target.entity_type()
    }

    pub fn name(&self) -> Cow<str> {
        let mut name = Cow::Borrowed(self.target.name);
        for x in &self.xforms.ty_alias {
            x.name(&mut name)
        }
        name
    }

    pub fn target_ty(&self) -> Type {
        Type::new(&self.target.target_ty, &self.xforms.entity_id)
    }

    pub fn attributes(&self) -> Attributes {
        Attributes::new(&self.target.attributes, &self.xforms.attr)
    }
}

#[cfg(test)]
mod tests {
    use crate::model::EntityId;
    use crate::test_util::executor::TestExecutor;
    use crate::view::tests::TestRenamer;
    use crate::view::Transformer;

    #[test]
    fn name() {
        let mut exe = TestExecutor::new("type TypeAlias = ns0::ns1::Type0;");
        let model = exe.model();
        let view = model.view().with_ty_alias_transform(TestRenamer {});
        let root = view.api();
        let alias = root
            .find_ty_alias(&EntityId::try_from("alias:TypeAlias").unwrap())
            .unwrap();

        assert_eq!(alias.name(), TestRenamer::renamed("TypeAlias"));
    }
}
