use crate::model::chunk;
use itertools::Itertools;
use std::borrow::Cow;

/// Additional metadata attached to entities.
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct Attributes<'a> {
    pub chunk: Option<chunk::Attribute>,
    pub comments: Vec<Comment<'a>>,
}

#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct Comment<'a> {
    lines: Vec<Cow<'a, str>>,
}

impl<'a> Attributes<'a> {
    pub fn with_comments(comments: Vec<Comment<'a>>) -> Self {
        Self {
            comments,
            ..Default::default()
        }
    }

    pub fn merge(&mut self, other: Self) {
        match (&mut self.chunk, other.chunk) {
            (Some(chunk), Some(mut other)) => chunk
                .relative_file_paths
                .append(&mut other.relative_file_paths),
            (None, Some(other)) => self.chunk = Some(other),
            _ => {}
        }
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
