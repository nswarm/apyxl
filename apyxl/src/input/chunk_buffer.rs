use itertools::Itertools;

use crate::input::Input;
use crate::model::Chunk;

/// Stores data across multiple in-memory chunks.
#[derive(Default)]
pub struct ChunkBuffer {
    chunks: Vec<(Chunk, String)>,
}

impl ChunkBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_chunk(&mut self, chunk: Chunk, data: impl ToString) {
        self.chunks.push((chunk, data.to_string()))
    }
}

impl Input for ChunkBuffer {
    fn chunks(&self) -> Vec<(&Chunk, &str)> {
        self.chunks
            .iter()
            .map(|(c, d)| (c, d.as_str()))
            .collect_vec()
    }
}
