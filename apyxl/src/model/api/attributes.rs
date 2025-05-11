use std::borrow::Cow;

use itertools::Itertools;

use crate::model::{chunk, EntityId};

/// Additional metadata attached to entities.
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct Attributes<'a> {
    pub chunk: Option<chunk::Attribute>,
    pub entity_id: EntityId,
    pub comments: Vec<Comment<'a>>,
    pub user: Vec<User<'a>>,
}

pub trait AttributesHolder {
    fn attributes(&self) -> &Attributes;
}

#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct Comment<'a> {
    lines: Vec<Cow<'a, str>>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct User<'a> {
    pub name: Cow<'a, str>,
    pub data: Vec<UserData<'a>>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UserData<'a> {
    pub key: Option<&'a str>,
    pub value: &'a str,
}

impl<'a> Attributes<'a> {
    pub fn merge(&mut self, other: Self) {
        self.merge_chunks(other.chunk);
        self.merge_comments(other.comments);
        self.merge_user(other.user);
        // Note: entity_id should typically be equivalent, but in the case where it's not it
        // makes sense to keep the current entity_id.
    }

    fn merge_chunks(&mut self, other: Option<chunk::Attribute>) {
        match (&mut self.chunk, other) {
            (Some(chunk), Some(mut other)) => chunk
                .relative_file_paths
                .append(&mut other.relative_file_paths),
            (None, Some(other)) => self.chunk = Some(other),
            _ => {}
        }
    }

    fn merge_comments(&mut self, mut other: Vec<Comment<'a>>) {
        self.comments.append(&mut other);
    }

    fn merge_user(&mut self, mut other: Vec<User<'a>>) {
        self.user.append(&mut other);
    }
}

impl<'a> Comment<'a> {
    pub fn unowned<S: AsRef<str>>(lines: &'a [S]) -> Self {
        Self {
            lines: lines
                .iter()
                .map(|s| Cow::Borrowed(s.as_ref()))
                .collect_vec(),
        }
    }

    pub fn lines(&self) -> impl Iterator<Item = &Cow<'a, str>> {
        self.lines.iter()
    }

    pub fn lines_mut(&mut self) -> impl Iterator<Item = &mut Cow<'a, str>> {
        self.lines.iter_mut()
    }
}

impl<'a> From<Vec<Cow<'a, str>>> for Comment<'a> {
    fn from(value: Vec<Cow<'a, str>>) -> Self {
        Self { lines: value }
    }
}

impl<'a> From<Vec<&'a str>> for Comment<'a> {
    fn from(value: Vec<&'a str>) -> Self {
        Self {
            lines: value.into_iter().map(|s| Cow::Borrowed(s)).collect_vec(),
        }
    }
}

impl<'a> User<'a> {
    pub fn new(name: &'a str, data: Vec<UserData<'a>>) -> Self {
        Self {
            name: Cow::Borrowed(name),
            data,
        }
    }

    pub fn new_flag(name: &'a str) -> Self {
        Self {
            name: Cow::Borrowed(name),
            data: vec![],
        }
    }
}

impl<'a> UserData<'a> {
    pub fn new(key: Option<&'a str>, value: &'a str) -> Self {
        Self { key, value }
    }
}

#[cfg(test)]
mod tests {
    use crate::model::attributes::User;
    use crate::model::{Attributes, Comment};

    mod merge_chunks {
        use crate::model::{chunk, Attributes};
        use std::path::PathBuf;

        #[test]
        fn none_plus_some() {
            let expected = chunk::Attribute {
                relative_file_paths: vec![PathBuf::from("asdf")],
            };
            let mut attr = Attributes {
                chunk: None,
                ..Default::default()
            };
            let other = Attributes {
                chunk: Some(expected.clone()),
                ..Default::default()
            };
            attr.merge(other);
            assert_eq!(attr.chunk, Some(expected));
        }

        #[test]
        fn some_plus_none() {
            let expected = chunk::Attribute {
                relative_file_paths: vec![PathBuf::from("asdf")],
            };
            let mut attr = Attributes {
                chunk: Some(expected.clone()),
                ..Default::default()
            };
            let other = Attributes {
                chunk: None,
                ..Default::default()
            };
            attr.merge(other);
            assert_eq!(attr.chunk, Some(expected));
        }

        #[test]
        fn some_plus_some() {
            let expected = chunk::Attribute {
                relative_file_paths: vec![PathBuf::from("asdf")],
            };
            let mut attr = Attributes {
                chunk: Some(expected.clone()),
                ..Default::default()
            };
            let other = Attributes {
                chunk: Some(expected.clone()),
                ..Default::default()
            };
            attr.merge(other);
            assert_eq!(
                attr.chunk,
                Some(chunk::Attribute {
                    relative_file_paths: vec![PathBuf::from("asdf"), PathBuf::from("asdf")]
                })
            );
        }
    }

    #[test]
    fn merge_comments() {
        let mut attr = Attributes {
            comments: vec![Comment::unowned(&["hi"])],
            ..Default::default()
        };
        let other = Attributes {
            comments: vec![Comment::unowned(&["there"])],
            ..Default::default()
        };
        attr.merge(other);
        assert_eq!(
            attr.comments,
            vec![Comment::unowned(&["hi"]), Comment::unowned(&["there"])],
        );
    }

    #[test]
    fn merge_user() {
        let mut attr = Attributes {
            user: vec![User::new_flag("hi")],
            ..Default::default()
        };
        let other = Attributes {
            user: vec![User::new_flag("there")],
            ..Default::default()
        };
        attr.merge(other);
        assert_eq!(
            attr.user,
            vec![User::new_flag("hi"), User::new_flag("there")],
        );
    }
}
