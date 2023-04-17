use std::io::{stdin, Read};

use crate::{input, model};
use anyhow::Result;

use crate::input::Input;

pub struct StdIn {
    chunk: input::Chunk,
}

impl StdIn {
    /// Pulls all available data from stdin immediately on creation.
    pub fn new() -> Result<Self> {
        let mut s = Self {
            chunk: input::Chunk {
                data: String::new(),
                chunk: model::Chunk::default(),
            },
        };
        stdin().read_to_string(&mut s.chunk.data)?;
        Ok(s)
    }
}

impl Input for StdIn {
    fn next_chunk(&self) -> Option<&input::Chunk> {
        Some(&self.chunk)
    }
}
