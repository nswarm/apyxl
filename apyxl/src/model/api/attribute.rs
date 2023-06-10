use std::borrow::Cow;

use itertools::Itertools;

use crate::model::chunk;

/// Additional metadata attached to entities.
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct Attributes<'a> {
    pub chunk: Option<chunk::Attribute>,
    pub comments: Vec<Comment<'a>>,
    pub user: Vec<User<'a>>,
}

#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct Comment<'a> {
    lines: Vec<Cow<'a, str>>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct User<'a> {
    pub name: &'a str,
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
        Self { name, data }
    }

    pub fn new_flag(name: &'a str) -> Self {
        Self { name, data: vec![] }
    }
}

impl<'a> UserData<'a> {
    pub fn new(key: Option<&'a str>, value: &'a str) -> Self {
        Self { key: None, value }
    }
}

#[cfg(test)]
mod tests {
    use crate::model::Attributes;

    #[test]
    fn merge_chunks() {
        let attr = Attributes {
            chunk: None,
            ..Default::default()
        };

        let other = Attributes {
            chunk: None,
            ..Default::default()
        };
        todo!()
    }
}
