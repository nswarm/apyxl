use std::cell::RefCell;

use crate::input::Input;
use crate::{input, model};

/// Stores all data in a single `chunk`.
#[derive(Default)]
pub struct Buffer {
    chunk: input::Chunk,
    read: RefCell<bool>,
}

impl Buffer {
    pub fn new(data: impl ToString) -> Self {
        Self {
            chunk: input::Chunk {
                data: data.to_string(),
                chunk: model::Chunk::default(),
            },
            read: RefCell::new(false),
        }
    }
}

impl Input for Buffer {
    fn next_chunk(&self) -> Option<&input::Chunk> {
        if *self.read.borrow() {
            return None;
        }
        *self.read.borrow_mut() = true;
        Some(&self.chunk)
    }
}
