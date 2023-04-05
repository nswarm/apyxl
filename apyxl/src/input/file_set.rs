use anyhow::{Context, Result};
use std::cell::RefCell;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::input::Input;

/// Input from one or more files in a file system.
#[derive(Default)]
pub struct FileSet {
    files: Vec<String>,
    cursor: RefCell<usize>,
}

impl FileSet {
    /// Loads all files into memory. Errors if any fail to be read.
    pub fn new<P: AsRef<Path>>(files: &[P]) -> Result<Self> {
        let mut s = Self {
            files: vec![],
            cursor: RefCell::new(0),
        };
        for file in files {
            let mut content = String::new();
            let file = file.as_ref();
            File::open(file)
                .with_context(|| format!("Failed to open input file for read: {}", file.display()))?
                .read_to_string(&mut content)
                .with_context(|| format!("Failed to read file to string: {}", file.display()))?;
            s.files.push(content);
        }
        Ok(s)
    }
}

impl Input for FileSet {
    fn next_chunk(&self) -> Option<&str> {
        let cursor = *self.cursor.borrow();
        if cursor >= self.files.len() {
            return None;
        }
        let file = &self.files[cursor];
        *self.cursor.borrow_mut() = cursor + 1;
        Some(file)
    }
}

#[cfg(test)]
mod tests {
    use crate::input::FileSet;
    use crate::Input;
    use anyhow::Result;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn reads_each_file_as_chunk() -> Result<()> {
        let dir = tempdir()?;
        let path0 = dir.path().join("test0");
        let path1 = dir.path().join("test1");
        File::create(&path0)?.write_all("test0".as_bytes())?;
        File::create(&path1)?.write_all("test1".as_bytes())?;
        let input = FileSet::new(&[path0.to_str().unwrap(), path1.to_str().unwrap()])?;
        assert_eq!(input.next_chunk(), Some("test0"));
        assert_eq!(input.next_chunk(), Some("test1"));
        assert_eq!(input.next_chunk(), None);
        Ok(())
    }

    #[test]
    fn returns_none_when_empty() -> Result<()> {
        let input = FileSet::new::<&str>(&[])?;
        assert_eq!(input.next_chunk(), None);
        Ok(())
    }

    #[test]
    fn missing_file_errors() {
        assert!(FileSet::new(&["i/do/not/exist"]).is_err());
    }
}
