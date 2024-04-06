use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};

use crate::model::Chunk;
use crate::Output;

/// Creates a file for each [Chunk] within the `output_root` using the [Chunk]'s `relative_file_path`.
/// Any data written without a [Chunk] is ignored.
#[derive(Debug, Default)]
pub struct FileSet {
    output_root: PathBuf,
    current: Option<(Chunk, File)>,
}

impl FileSet {
    pub fn new<P: Into<PathBuf>>(output_root: P) -> Result<Self> {
        let output_root = output_root.into();
        fs::create_dir_all(&output_root)?;
        let dir_metadata = fs::metadata(&output_root).context("output_root")?;
        if !dir_metadata.is_dir() {
            return Err(anyhow!("specified 'output_root' must be a directory"));
        }
        if fs::read_dir(&output_root)?.count() > 0 {
            return Err(anyhow!("specified 'output_root' must be empty"));
        }
        Ok(Self {
            output_root,
            current: None,
        })
    }
}

impl Output for FileSet {
    /// Opens a new File at `chunk`'s `relative_file_path` and sets it as the current chunk. Any
    /// File open for the current chunk will be closed first.
    fn write_chunk(&mut self, chunk: &Chunk) -> Result<()> {
        let path = chunk.relative_file_path.as_ref().ok_or_else(|| {
            anyhow!("all chunks must have file paths when generating to a FileSet")
        })?;
        let path = self.output_root.join(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        self.current = Some((chunk.clone(), File::create(path)?));
        Ok(())
    }

    fn write(&mut self, data: &str) -> Result<()> {
        if let Some((_, file)) = &mut self.current {
            file.write_all(data.as_bytes())?;
        }
        Ok(())
    }

    fn write_char(&mut self, data: char) -> Result<()> {
        if let Some((_, file)) = &mut self.current {
            file.write_all(&[data as u8])?;
        }
        Ok(())
    }

    fn newline(&mut self) -> Result<()> {
        self.write_char('\n')
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use anyhow::Result;
    use tempfile::tempdir;

    use crate::model::Chunk;
    use crate::output::file_set::FileSet;
    use crate::Output;

    mod new {
        use std::fs::File;

        use anyhow::Result;
        use tempfile::tempdir;

        use crate::output::FileSet;

        #[test]
        fn success() -> Result<()> {
            let root = tempdir()?;
            assert!(FileSet::new(root.path()).is_ok());
            Ok(())
        }

        #[test]
        fn path_doesnt_exist_is_created() -> Result<()> {
            let root = tempdir()?;
            let path = root.path().join("asdf");
            assert!(FileSet::new(&path).is_ok());
            assert!(path.exists());
            Ok(())
        }

        #[test]
        fn path_is_not_dir_errors() -> Result<()> {
            let root = tempdir()?;
            let file_path = root.path().join("asdf");
            File::create(&file_path)?;
            assert!(FileSet::new(file_path).is_err());
            Ok(())
        }

        #[test]
        fn path_not_empty_errors() -> Result<()> {
            let root = tempdir()?;
            File::create(root.path().join("some_file"))?;
            assert!(FileSet::new(root.path()).is_err());
            Ok(())
        }
    }

    mod chunk {
        use std::fs;

        use anyhow::Result;
        use tempfile::tempdir;

        use crate::model::Chunk;
        use crate::output::FileSet;
        use crate::Output;

        #[test]
        fn creates_file_per_chunk() -> Result<()> {
            let root = tempdir()?;
            {
                let mut output = FileSet::new(root.path())?;
                let chunks = vec![
                    Chunk::with_relative_file_path(root.path().join("a")),
                    Chunk::with_relative_file_path(root.path().join("b")),
                    Chunk::with_relative_file_path(root.path().join("c")),
                ];
                for chunk in chunks {
                    output.write_chunk(&chunk)?;
                    output.write(
                        &chunk
                            .relative_file_path
                            .unwrap()
                            .file_name()
                            .unwrap()
                            .to_string_lossy(),
                    )?;
                }
            } // close fileset
            assert_eq!(fs::read_to_string(root.path().join("a"))?, "a");
            assert_eq!(fs::read_to_string(root.path().join("b"))?, "b");
            assert_eq!(fs::read_to_string(root.path().join("c"))?, "c");
            Ok(())
        }

        #[test]
        fn creates_full_file_path_relative_to_root() -> Result<()> {
            let root = tempdir()?;
            {
                let mut output = FileSet::new(root.path())?;
                let chunk = Chunk::with_relative_file_path(root.path().join("a/b/c/d"));
                output.write_chunk(&chunk)?;
            } // close fileset
            assert!(root.path().join("a/b/c/d").exists());
            Ok(())
        }

        #[test]
        fn chunk_without_path_errors() -> Result<()> {
            let root = tempdir()?;
            let mut output = FileSet::new(root.path())?;
            let chunk = Chunk::default();
            assert!(output.write_chunk(&chunk).is_err());
            Ok(())
        }
    }

    #[test]
    fn write_to_current_chunk() -> Result<()> {
        let root = tempdir()?;
        let mut output = FileSet::new(root.path())?;
        let chunk = Chunk::with_relative_file_path(root.path().join("file"));
        output.write_chunk(&chunk)?;
        output.write("content")?;
        output.write_char('!')?;
        assert_eq!(fs::read_to_string(root.path().join("file"))?, "content!");
        Ok(())
    }

    #[test]
    fn write_without_current_chunk_is_ignored() -> Result<()> {
        let root = tempdir()?;
        let mut output = FileSet::new(root.path())?;
        assert!(output.write("content").is_ok());
        assert!(output.write_char('!').is_ok());
        Ok(())
    }
}
