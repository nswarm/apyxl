use std::cell::RefCell;

use crate::input::Input;

/// Stores all data in a single `chunk`.
#[derive(Default)]
pub struct Buffer {
    data: String,
    read: RefCell<bool>,
}

impl Buffer {
    pub fn new(data: impl ToString) -> Self {
        Self {
            data: data.to_string(),
            read: RefCell::new(false),
        }
    }
}

impl Input for Buffer {
    fn next_chunk(&self) -> Option<&str> {
        if *self.read.borrow() {
            return None;
        }
        *self.read.borrow_mut() = true;
        Some(&self.data)
    }
}
