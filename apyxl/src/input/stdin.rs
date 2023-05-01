use std::io::{stdin, Read};

use anyhow::Result;

use crate::input::{Data, Input};
use crate::model::Chunk;

pub struct StdIn {
    chunk: Chunk,
    data: Data,
}

impl StdIn {
    /// Pulls all available data from stdin immediately on creation.
    pub fn new() -> Result<Self> {
        let mut s = Self {
            data: String::new(),
            chunk: Chunk::default(),
        };
        stdin().read_to_string(&mut s.data)?;
        Ok(s)
    }
}

impl Input for StdIn {
    fn chunks(&self) -> Vec<(&Chunk, &Data)> {
        vec![(&self.chunk, &self.data)]
    }
}
