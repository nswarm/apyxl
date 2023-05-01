use crate::input::{Data, Input};
use crate::model::Chunk;

/// Stores all data in a single in-memory chunk.
#[derive(Default)]
pub struct Buffer {
    chunk: Chunk,
    data: Data,
}

impl Buffer {
    pub fn new(data: impl ToString) -> Self {
        Self {
            chunk: Chunk::default(),
            data: data.to_string(),
        }
    }
}

impl Input for Buffer {
    fn chunks(&self) -> Vec<(&Chunk, &Data)> {
        vec![(&self.chunk, &self.data)]
    }
}
