use std::path::PathBuf;

use chumsky::container::Seq;

use crate::model;
use crate::model::{Attributes, EntityId};
use crate::view::NamespaceTransform;

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

#[derive(Debug)]
pub struct ChunkFilter {
    pub relative_file_path: PathBuf,
}

impl NamespaceTransform for ChunkFilter {
    fn filter_namespace(&self, value: &model::Namespace) -> bool {
        filter_attributes(&value.attributes, &self.relative_file_path)
    }

    fn filter_dto(&self, value: &model::Dto) -> bool {
        filter_attributes(&value.attributes, &self.relative_file_path)
    }

    fn filter_rpc(&self, value: &model::Rpc) -> bool {
        filter_attributes(&value.attributes, &self.relative_file_path)
    }
}

fn filter_attributes(attr: &Attributes, relative_file_path: &PathBuf) -> bool {
    attr.chunk
        .as_ref()
        .map(|chunk| chunk.relative_file_paths.contains(relative_file_path))
        .is_some()
}
