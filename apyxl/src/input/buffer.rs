use std::cell::RefCell;
use std::path::PathBuf;

use crate::input::{Chunk, Input};

/// Stores all data in a single `chunk`.
#[derive(Default)]
pub struct Buffer {
    chunk: Chunk,
    read: RefCell<bool>,
}

impl Buffer {
    pub fn new(data: impl ToString) -> Self {
        Self {
            chunk: Chunk {
                data: data.to_string(),
                relative_file_path: PathBuf::new(),
            },
            read: RefCell::new(false),
        }
    }
}

impl Input for Buffer {
    fn next_chunk(&self) -> Option<&Chunk> {
        if *self.read.borrow() {
            return None;
        }
        *self.read.borrow_mut() = true;
        Some(&self.chunk)
    }
}
