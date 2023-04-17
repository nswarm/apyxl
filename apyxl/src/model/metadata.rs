use crate::model::EntityId;
use std::borrow::Cow;
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct Metadata<'a> {
    pub chunks: Vec<Chunk<'a>>,
}

#[derive(Debug, Default)]
pub struct Chunk<'a> {
    /// The namespace that all entities within the chunk reside.
    /// Entities will still need to be filtered by the [Chunk::ATTRIBUTE].
    pub root_namespace: EntityId<'a>,

    /// See [crate::input::Chunk].
    pub relative_file_path: PathBuf,
}

impl Chunk<'_> {
    /// Attribute key that API entities will have to associated them with a [Chunk].
    /// The attribute value should be the string from [Chunk::relative_file_path_str]
    pub const ATTRIBUTE: &'static str = "chunk_relative_file_path";

    pub fn relative_file_path_str(&self) -> Cow<str> {
        self.relative_file_path.to_string_lossy()
    }
}
