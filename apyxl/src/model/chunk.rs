use crate::model::EntityId;
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct Metadata<'a> {
    /// The namespace that all entities within the chunk reside.
    /// Entities will still need to be filtered by the [Attribute].
    pub root_namespace: EntityId<'a>,

    /// See [crate::input::Chunk].
    pub relative_file_path: Option<PathBuf>,
}

#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct Attribute {
    /// Some entities (namespaces) can exists in more than one chunk.
    /// See [crate::input::Chunk].
    pub relative_file_paths: Vec<PathBuf>,
}
