use crate::model;
use crate::model::{chunk, Comment};
use crate::view::{EntityId, EntityIdTransform};
use dyn_clone::DynClone;
use std::fmt::Debug;

#[derive(Debug, Copy, Clone)]
pub struct Attributes<'v, 'a> {
    target: &'v model::Attributes<'a>,
    xforms: &'v Vec<Box<dyn AttributeTransform>>,
    entity_id_xforms: &'v Vec<Box<dyn EntityIdTransform>>,
}

impl<'v, 'a> Attributes<'v, 'a> {
    pub fn new(
        target: &'v model::Attributes<'a>,
        xforms: &'v Vec<Box<dyn AttributeTransform>>,
        entity_id_xforms: &'v Vec<Box<dyn EntityIdTransform>>,
    ) -> Self {
        Self {
            target,
            xforms,
            entity_id_xforms,
        }
    }

    pub fn chunk(&self) -> Option<&chunk::Attribute> {
        self.target.chunk.as_ref()
    }

    pub fn entity_id(&self) -> EntityId {
        EntityId::new(&self.target.entity_id, self.entity_id_xforms)
    }

    pub fn comments(&self) -> Vec<Comment<'a>> {
        let mut comments = self.target.comments.clone();
        for x in self.xforms {
            x.comments(&mut comments);
        }
        comments
    }

    pub fn user(&self) -> Vec<model::attributes::User<'a>> {
        let mut attrs = self.target.user.clone();
        for x in self.xforms {
            x.user(&mut attrs);
        }
        attrs
    }
}

pub trait AttributeTransform: Debug + DynClone {
    fn comments(&self, comment: &mut Vec<Comment>);
    fn user(&self, attr: &mut Vec<model::attributes::User>);
}

dyn_clone::clone_trait_object!(AttributeTransform);

#[cfg(test)]
mod tests {
    use crate::model;
    use crate::model::attributes::User;
    use crate::test_util::executor::TestExecutor;
    use crate::view::{AttributeTransform, Transformer};
    use std::borrow::Cow;

    #[test]
    fn transform() {
        let mut exe = TestExecutor::new(
            r#"
                    // This comment has a bad_word
                    // bad_word bad_word bad_word
                    #[some_attr, bad_word_attr]
                    struct dto {}
                "#,
        );
        let model = exe.build();
        let view = model
            .view()
            .with_attribute_transform(WordFilterTransform {});
        let root = view.api();
        let dto = root
            .find_dto(&model::EntityId::try_from("d:dto").unwrap())
            .unwrap();
        let attr = dto.attributes();
        assert_eq!(
            attr.comments(),
            vec![model::Comment::unowned(&[
                "This comment has a <3",
                "<3 <3 <3"
            ])],
        );
        assert_eq!(attr.user()[0].name.as_ref(), "some_attr");
        assert_eq!(attr.user()[1].name.as_ref(), "nice_attr");
    }

    #[derive(Debug, Clone)]
    struct WordFilterTransform {}
    impl AttributeTransform for WordFilterTransform {
        fn comments(&self, comments: &mut Vec<model::Comment>) {
            comments.iter_mut().for_each(|comment| {
                comment.lines_mut().for_each(|line| {
                    if line.contains("bad_word") {
                        *line = Cow::Owned(line.replace("bad_word", "<3"));
                    }
                });
            });
        }

        fn user(&self, attr: &mut Vec<User>) {
            attr.iter_mut().for_each(|attr| {
                if attr.name.contains("bad_word") {
                    attr.name = Cow::Owned(attr.name.replace("bad_word", "nice"));
                }
            });
        }
    }
}
