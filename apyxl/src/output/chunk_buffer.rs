use crate::model::chunk::Chunk;
use crate::output::{Buffer, Output};
use anyhow::{anyhow, Result};
use std::collections::HashMap;

/// A collection of [Buffer]s, one per chunk, indexed by their file path as a string.
#[derive(Debug, Default)]
pub struct ChunkBuffer {
    data: HashMap<String, Buffer>,
    latest: String,
}

impl ChunkBuffer {
    pub fn data(&self, key: &str) -> Option<&Buffer> {
        self.data.get(key)
    }

    fn latest_mut(&mut self) -> Option<&mut Buffer> {
        self.data.get_mut(&self.latest)
    }
}

impl Output for ChunkBuffer {
    fn write_chunk(&mut self, chunk: &Chunk) -> Result<()> {
        let latest = match &chunk.relative_file_path {
            None => return Err(anyhow!("ChunkBuffer chunks must have relative file paths")),
            Some(path) => path.to_string_lossy(),
        };
        self.data.insert(latest.to_string(), Buffer::default());
        self.latest = latest.to_string();
        Ok(())
    }

    fn write(&mut self, data: &str) -> Result<()> {
        if let Some(buffer) = self.latest_mut() {
            buffer.write(data)?;
            Ok(())
        } else {
            err_no_chunks()
        }
    }

    fn write_char(&mut self, data: char) -> Result<()> {
        if let Some(buffer) = self.latest_mut() {
            buffer.write_char(data)?;
            Ok(())
        } else {
            err_no_chunks()
        }
    }

    fn newline(&mut self) -> Result<()> {
        if let Some(buffer) = self.latest_mut() {
            buffer.newline()?;
            Ok(())
        } else {
            err_no_chunks()
        }
    }
}

fn err_no_chunks() -> Result<()> {
    Err(anyhow!("must write_chunk before writing to a ChunkBuffer"))
}

#[cfg(test)]
mod tests {
    use crate::model::Chunk;
    use crate::output::ChunkBuffer;
    use crate::Output;
    use anyhow::Result;

    #[test]
    fn write_separate_chunks() -> Result<()> {
        let mut output = ChunkBuffer::default();
        output.write_chunk(&Chunk::with_relative_file_path("some/path1"))?;
        output.write("chunk 1 data")?;
        output.write_chunk(&Chunk::with_relative_file_path("path2"))?;
        output.write("chunk 2 data")?;
        assert!(output.data("some/path1").is_some());
        assert!(output.data("path2").is_some());
        assert!(output.data("noexist").is_none());
        assert_eq!(output.data("some/path1").unwrap().data(), "chunk 1 data");
        assert_eq!(output.data("path2").unwrap().data(), "chunk 2 data");
        Ok(())
    }

    #[test]
    fn write_str() -> Result<()> {
        let mut output = ChunkBuffer::default();
        output.write_chunk(&Chunk::with_relative_file_path("chunk"))?;
        output.write("asdf")?;
        assert_eq!(output.data("chunk").unwrap().to_string(), "asdf");
        Ok(())
    }

    #[test]
    fn write_char() -> Result<()> {
        let mut output = ChunkBuffer::default();
        output.write_chunk(&Chunk::with_relative_file_path("chunk"))?;
        output.write_char(':')?;
        assert_eq!(output.data("chunk").unwrap().to_string(), ":");
        Ok(())
    }

    #[test]
    fn write_appends() -> Result<()> {
        let mut output = ChunkBuffer::default();
        output.write_chunk(&Chunk::with_relative_file_path("chunk"))?;
        output.write("abc")?;
        output.write_char('d')?;
        output.write("efg")?;
        assert_eq!(output.data("chunk").unwrap().to_string(), "abcdefg");
        Ok(())
    }
}
