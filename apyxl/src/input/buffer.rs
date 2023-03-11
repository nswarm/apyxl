use crate::input::Input;

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
    fn data(&self) -> &str {
        &self.data
    }
}
