use std::path::PathBuf;

use chumsky::container::Seq;

use crate::model;
use crate::model::{Attributes, EntityId};
use crate::view::NamespaceTransform;

#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct Chunk {
    /// Relative path including file name from a common root path shared by the other [Chunk]s from
    /// the [Input]. Typically used by a [crate::Generator] to determine where to put the final file
    /// for this data, and how to refer to it from other files for includes/imports.
    pub relative_file_path: Option<PathBuf>,
}

impl Chunk {
    pub fn with_relative_file_path(relative_file_path: PathBuf) -> Self {
        Self {
            relative_file_path: Some(relative_file_path),
        }
    }
}

#[derive(Debug, Default)]
pub struct Metadata<'a> {
    /// The namespace that all entities within the chunk reside.
    /// Entities will still need to be filtered by the [Attribute] via the [ChunkFilter]
    pub root_namespace: EntityId<'a>,

    /// Information stored about the chunk.
    pub chunk: Chunk,
}

#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct Attribute {
    /// Some entities (namespaces) can exists in more than one chunk.
    pub relative_file_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
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
