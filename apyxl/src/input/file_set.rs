use std::cell::RefCell;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::input::{Chunk, Input};

/// Input from one or more files in a file system.
#[derive(Default)]
pub struct FileSet {
    chunks: Vec<Chunk>,
    cursor: RefCell<usize>,
}

impl FileSet {
    /// Loads all files into memory. Errors if any fail to be read.
    pub fn new<R, P>(root_path: R, relative_paths: &[P]) -> Result<Self>
    where
        R: AsRef<Path>,
        P: AsRef<Path>,
    {
        let mut s = Self {
            chunks: vec![],
            cursor: RefCell::new(0),
        };
        for relative_path in relative_paths {
            let relative_file_path = relative_path.as_ref().to_path_buf();

            let file_path = root_path.as_ref().join(&relative_file_path);
            let content = fs::read_to_string(&file_path).with_context(|| {
                format!("Failed to read file to string: {}", file_path.display())
            })?;
            s.chunks.push(Chunk {
                data: content.to_string(),
                relative_file_path,
            });
        }
        Ok(s)
    }
}

impl Input for FileSet {
    fn next_chunk(&self) -> Option<&Chunk> {
        let cursor = *self.cursor.borrow();
        if cursor >= self.chunks.len() {
            return None;
        }
        let file = &self.chunks[cursor];
        *self.cursor.borrow_mut() = cursor + 1;
        Some(file)
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Write;
    use std::path::{Path, PathBuf};

    use anyhow::Result;
    use tempfile::tempdir;

    use crate::input::FileSet;
    use crate::Input;

    #[test]
    fn reads_each_file_as_chunk() -> Result<()> {
        let root = tempdir()?;
        let path0 = root.path().join("test0");
        let path1 = root.path().join("test1");
        File::create(&path0)?.write_all("test0".as_bytes())?;
        File::create(&path1)?.write_all("test1".as_bytes())?;
        let input = FileSet::new("", &[path0, path1])?;
        assert_eq!(input.next_chunk().map(|c| c.data.as_str()), Some("test0"));
        assert_eq!(input.next_chunk().map(|c| c.data.as_str()), Some("test1"));
        assert_eq!(input.next_chunk().map(|c| c.data.as_str()), None);
        Ok(())
    }

    #[test]
    fn passes_relative_path_to_chunk() -> Result<()> {
        let root = tempdir()?;
        let path0 = create_file_in(root.path(), "test0");
        let path1 = create_file_in(root.path(), "test1");
        let input = FileSet::new(&root, &[&path0, &path1])?;
        assert_eq!(
            input.next_chunk().map(|c| c.relative_file_path.clone()),
            Some(path0)
        );
        assert_eq!(
            input.next_chunk().map(|c| c.relative_file_path.clone()),
            Some(path1)
        );
        Ok(())
    }

    fn create_file_in(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        File::create(&path).unwrap();
        path
    }

    #[test]
    fn returns_none_when_empty() -> Result<()> {
        let input = FileSet::new::<&str, &str>("", &[])?;
        assert_eq!(input.next_chunk().map(|c| c.data.as_str()), None);
        Ok(())
    }

    #[test]
    fn missing_file_errors() {
        assert!(FileSet::new("", &["i/do/not/exist"]).is_err());
    }
}
