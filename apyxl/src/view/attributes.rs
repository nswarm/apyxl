use crate::model;
use crate::model::{chunk, Comment};
use dyn_clone::DynClone;
use std::fmt::Debug;

#[derive(Debug, Copy, Clone)]
pub struct Attributes<'v, 'a> {
    target: &'v model::Attributes<'a>,
    xforms: &'v Vec<Box<dyn AttributeTransform>>,
}

impl<'v, 'a> Attributes<'v, 'a> {
    pub fn new(
        target: &'v model::Attributes<'a>,
        xforms: &'v Vec<Box<dyn AttributeTransform>>,
    ) -> Self {
        Self { target, xforms }
    }

    pub fn chunk(&self) -> Option<&chunk::Attribute> {
        self.target.chunk.as_ref()
    }

    pub fn comments(&self) -> Vec<Comment<'a>> {
        let mut comments = self.target.comments.clone();
        for x in self.xforms {
            x.comments(&mut comments)
        }
        comments
    }
}

pub trait AttributeTransform: Debug + DynClone {
    fn comments(&self, comment: &mut Vec<Comment>);
}

dyn_clone::clone_trait_object!(AttributeTransform);

#[cfg(test)]
mod tests {
    use crate::model::{Comment, EntityId};
    use crate::test_util::executor::TestExecutor;
    use crate::view::{AttributeTransform, Transformer};
    use std::borrow::Cow;

    #[test]
    fn attr_transform() {
        let mut exe = TestExecutor::new(
            r#"
                    // This comment has a bad_word
                    // bad_word bad_word bad_word
                    struct dto {}
                "#,
        );
        let model = exe.build();
        let view = model
            .view()
            .with_attribute_transform(CommentWordFilterTransform {});
        let root = view.api();
        let dto = root
            .find_dto(&EntityId::try_from("d:dto").unwrap())
            .unwrap();
        let attr = dto.attributes();
        assert_eq!(
            attr.comments(),
            vec![Comment::unowned(&["This comment has a <3", "<3 <3 <3"])],
        );
    }

    #[derive(Debug, Clone)]
    struct CommentWordFilterTransform {}
    impl AttributeTransform for CommentWordFilterTransform {
        fn comments(&self, comments: &mut Vec<Comment>) {
            comments.iter_mut().for_each(|comment| {
                comment.lines_mut().for_each(|line| {
                    if line.contains("bad_word") {
                        *line = Cow::Owned(line.replace("bad_word", "<3"))
                    }
                });
            });
        }
    }
}
