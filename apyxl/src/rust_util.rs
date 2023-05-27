use crate::model;
use std::borrow::Cow;
use std::path::{Path, PathBuf};

pub fn path_to_entity_id(path: &Path) -> model::EntityId {
    model::EntityId::new_unqualified_vec(path_iter(&namespace_path(path)))
}

/// Iterate over path as strings.
fn path_iter<'a>(path: &'a Path) -> impl Iterator<Item = Cow<'a, str>> + 'a {
    path.iter().map(|p| p.to_string_lossy())
}

/// Convert file path to rust module path, obeying rules for {lib,mod}.rs.
fn namespace_path(file_path: &Path) -> PathBuf {
    if file_path.ends_with("mod.rs") || file_path.ends_with("lib.rs") {
        file_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or(PathBuf::default())
    } else {
        file_path.with_extension("")
    }
}
