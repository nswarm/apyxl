use crate::model;
use crate::view::{Namespace, Transformer, Transforms};

/// A view into the [Model] starting at a specific [Namespace] with additional [Transforms].
#[derive(Debug, Clone)]
pub struct SubView<'a> {
    namespace_id: model::EntityId,
    namespace: &'a model::Namespace<'a>,
    xforms: Transforms,
}

impl<'a> SubView<'a> {
    pub fn new(
        namespace_id: model::EntityId,
        namespace: &'a model::Namespace<'a>,
        xforms: Transforms,
    ) -> Self {
        Self {
            namespace_id: namespace_id.with_qualified_namespaces(),
            namespace,
            xforms,
        }
    }

    pub fn root_id(&self) -> &model::EntityId {
        &self.namespace_id
    }

    pub fn namespace<'v>(&'v self) -> Namespace<'v, 'a> {
        Namespace::new(self.namespace, &self.xforms)
    }
}

impl Transformer for SubView<'_> {
    fn xforms(&mut self) -> &mut Transforms {
        &mut self.xforms
    }
}

#[cfg(test)]
mod tests {
    use crate::model;
    use itertools::Itertools;

    use crate::test_util::executor::TestExecutor;
    use crate::view::tests::TestFilter;
    use crate::view::{SubView, Transformer, Transforms};

    #[test]
    fn filters() {
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
        let sub_view = SubView::new(
            model::EntityId::default(),
            &model.api(),
            Transforms::default(),
        )
        .with_namespace_transform(TestFilter {});
        let namespace = sub_view.namespace();

        assert_eq!(namespace.namespaces().count(), 1);
        assert_eq!(namespace.dtos().count(), 1);
        assert_eq!(namespace.rpcs().count(), 1);

        assert_eq!(
            namespace.namespaces().collect_vec().get(0).unwrap().name(),
            "visible"
        );
        assert_eq!(
            namespace.dtos().collect_vec().get(0).unwrap().name(),
            "visible"
        );
        assert_eq!(
            namespace.rpcs().collect_vec().get(0).unwrap().name(),
            "visible"
        );
    }
}
