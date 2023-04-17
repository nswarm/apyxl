use crate::model;
use crate::view::{Attributes, EntityId, Field, Transforms};
use std::borrow::Cow;
use std::fmt::Debug;

/// A single Remote Procedure Call (RPC) within an [Api].
/// Wraps [model::Rpc].
#[derive(Debug, Copy, Clone)]
pub struct Rpc<'v, 'a> {
    target: &'v model::Rpc<'a>,
    xforms: &'v Transforms,
}

pub trait RpcTransform: Debug {
    fn name(&self, _: &mut Cow<str>) {}
    fn return_type(&self, _: &mut model::EntityId) {}

    /// `true`: included.
    /// `false`: excluded.
    fn filter_param(&self, _: &model::Field) -> bool {
        true
    }
}

impl<'v, 'a> Rpc<'v, 'a> {
    pub fn new(target: &'v model::Rpc<'a>, xforms: &'v Transforms) -> Self {
        Self { target, xforms }
    }

    pub fn name(&self) -> Cow<str> {
        let mut name = Cow::Borrowed(self.target.name);
        for x in &self.xforms.rpc {
            x.name(&mut name)
        }
        name
    }

    pub fn params(&'a self) -> impl Iterator<Item = Field<'v, 'a>> {
        self.target
            .params
            .iter()
            .filter(|param| self.filter_param(param))
            .map(move |param| {
                Field::new(
                    param,
                    &self.xforms.rpc_param,
                    &self.xforms.entity_id_xforms,
                    &self.xforms.attr_xforms,
                )
            })
    }

    pub fn return_type(&self) -> Option<EntityId> {
        self.target
            .return_type
            .as_ref()
            .map(|target| EntityId::new(target, &self.xforms.entity_id_xforms))
    }

    pub fn attributes(&self) -> Attributes {
        Attributes::new(&self.target.attributes, &self.xforms.attr_xforms)
    }

    fn filter_param(&self, param: &model::Field) -> bool {
        self.xforms.rpc.iter().all(|x| x.filter_param(param))
    }
}

#[cfg(test)]
mod tests {
    use crate::test_util::executor::TestExecutor;
    use crate::view::tests::{TestFilter, TestRenamer};
    use itertools::Itertools;

    #[test]
    fn name() {
        let mut exe = TestExecutor::new(
            r#"
                    mod ns0 {
                        fn rpc0() {}
                        mod ns1 {
                            fn rpc1() {}
                        }
                    }
                "#,
        );
        let model = exe.model();
        let view = model.view().with_rpc_transform(TestRenamer {});
        let root = view.api();

        assert_eq!(
            root.find_rpc(&["ns0", "rpc0"].into()).unwrap().name(),
            TestRenamer::renamed("rpc0")
        );
        assert_eq!(
            root.find_rpc(&["ns0", "ns1", "rpc1"].into())
                .unwrap()
                .name(),
            TestRenamer::renamed("rpc1")
        );
    }

    #[test]
    fn params() {
        let mut exe = TestExecutor::new(
            r#"
            fn rpc(visible0: Type, hidden: Type, visible1: Type) {}
            "#,
        );
        let model = exe.model();
        let view = model.view().with_rpc_transform(TestFilter {});
        let root = view.api();
        let rpc = root.find_rpc(&["rpc"].into()).unwrap();
        let params = rpc.params().map(|f| f.name().to_string()).collect_vec();

        assert_eq!(params, vec!["visible0", "visible1"]);
    }
}
