use std::io::{stdin, Read};
use std::path::PathBuf;

use crate::input::{Chunk, Input};
use anyhow::Result;

pub struct StdIn {
    chunk: Chunk,
}

impl StdIn {
    /// Pulls all available data from stdin immediately on creation.
    pub fn new() -> Result<Self> {
        let mut s = Self {
            chunk: Chunk {
                data: String::new(),
                relative_file_path: PathBuf::new(),
            },
        };
        stdin().read_to_string(&mut s.chunk.data)?;
        Ok(s)
    }
}

impl Input for StdIn {
    fn next_chunk(&self) -> Option<&Chunk> {
        Some(&self.chunk)
    }
}
