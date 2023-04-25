use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use walkdir::WalkDir;

use crate::input;
use crate::input::{Chunk, Data, Input};

/// Input from one or more files in a file system.
#[derive(Default)]
pub struct Glob {
    file_set: input::FileSet,
}

impl Glob {
    pub fn new<P: AsRef<Path>>(root_path: P, glob: &str) -> Result<Self> {
        Ok(Self {
            file_set: input::FileSet::new(&root_path, &walk_glob(root_path.as_ref(), glob)?)?,
        })
    }
}

impl Input for Glob {
    fn next_chunk(&self) -> Option<(&Chunk, &Data)> {
        self.file_set.next_chunk()
    }
}

fn walk_glob(root: &Path, glob: &str) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    let glob_path = root.join(glob);
    let glob = globset::Glob::new(
        glob_path
            .to_str()
            .ok_or_else(|| anyhow!("could not convert glob path '{:?}' to OS str", glob_path))?,
    )?
    .compile_matcher();
    for entry in WalkDir::new(root) {
        let entry = entry?;
        if entry.file_type().is_dir() {
            continue;
        }
        if !glob.is_match(entry.path()) {
            continue;
        }
        paths.push(entry.path().strip_prefix(root)?.to_path_buf());
    }
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::fs::File;
    use std::path::PathBuf;

    use anyhow::Result;
    use tempfile::tempdir;

    use crate::input::glob::walk_glob;

    #[test]
    fn happy_path() -> Result<()> {
        let root = tempdir()?;
        fs::create_dir_all(root.path().join("a/b"))?;
        fs::create_dir_all(root.path().join("a/c"))?;
        fs::create_dir_all(root.path().join("d/e"))?;
        let path0 = PathBuf::from("a/b/file0.rs");
        let path1 = PathBuf::from("a/b/file1.rs");
        let path2 = PathBuf::from("a/c/file2.rs");
        let path3 = PathBuf::from("d/e/file3.rs");
        File::create(root.path().join(&path0))?;
        File::create(root.path().join(&path1))?;
        File::create(root.path().join(&path2))?;
        File::create(root.path().join(&path3))?;
        let paths = walk_glob(root.path(), "a/**/*.rs")?;
        assert_eq!(paths, vec![path0, path1, path2,]);
        Ok(())
    }
}
