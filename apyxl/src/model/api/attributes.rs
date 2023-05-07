use crate::model::chunk;

/// Additional metadata attached to entities.
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct Attributes {
    pub chunk: Option<chunk::Attribute>,
}

impl Attributes {
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
