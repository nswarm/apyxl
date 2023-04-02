use crate::input::Input;

/// Stores all data in a single `chunk`.
#[derive(Default)]
pub struct Buffer {
    data: String,
}

impl Buffer {
    pub fn new(data: impl ToString) -> Self {
        Self {
            data: data.to_string(),
        }
    }
}

impl Input for Buffer {
    fn next_chunk(&mut self) -> Option<&str> {
        Some(&self.data)
    }
}
