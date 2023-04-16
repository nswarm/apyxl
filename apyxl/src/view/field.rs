use std::borrow::Cow;
use std::fmt::Debug;

use crate::model;
use crate::view::{AttributeTransform, Attributes, TypeRef, TypeRefTransform};

/// A pair of name and type that describe a named instance of a type e.g. within a [Dto] or [Rpc].
/// Wraps [model::Dto].
#[derive(Debug, Copy, Clone)]
pub struct Field<'v, 'a> {
    target: &'v model::Field<'a>,
    xforms: &'v Vec<Box<dyn FieldTransform>>,
    type_ref_xforms: &'v Vec<Box<dyn TypeRefTransform>>,
    attr_xforms: &'v Vec<Box<dyn AttributeTransform>>,
}

pub trait FieldTransform: Debug {
    fn name(&self, _: &mut Cow<str>) {}
}

impl<'v, 'a> Field<'v, 'a> {
    pub fn new(
        target: &'v model::Field<'a>,
        xforms: &'v Vec<Box<dyn FieldTransform>>,
        type_ref_xforms: &'v Vec<Box<dyn TypeRefTransform>>,
        attr_xforms: &'v Vec<Box<dyn AttributeTransform>>,
    ) -> Self {
        Self {
            target,
            xforms,
            type_ref_xforms,
            attr_xforms,
        }
    }

    pub fn name(&self) -> Cow<str> {
        let mut name = Cow::Borrowed(self.target.name);
        for x in self.xforms {
            x.name(&mut name)
        }
        name
    }

    pub fn ty(&self) -> TypeRef {
        TypeRef::new(&self.target.ty, self.type_ref_xforms)
    }

    pub fn attributes(&self) -> Attributes {
        Attributes::new(&self.target.attributes, self.attr_xforms)
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use crate::test_util::executor::TestExecutor;
    use crate::view::tests::TestRenamer;

    #[test]
    fn name() {
        let mut exe = TestExecutor::new(
            r#"
                struct dto {
                    field0: ns0::ns1::Type0,
                }
            "#,
        );
        let model = exe.model();
        let view = model.view().with_field_transform(TestRenamer {});
        let root = view.api();
        let dto = root.find_dto(&["dto"].into()).unwrap();
        let fields = dto.fields().collect_vec();
        let field = fields.get(0).unwrap();

        assert_eq!(field.name(), TestRenamer::renamed("field0"));
    }
}
