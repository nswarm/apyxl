use crate::input::Input;

/// Stores all data in a single `chunk`.
#[derive(Default)]
pub struct Buffer {
    data: String,
    read: bool,
}

impl Buffer {
    pub fn new(data: impl ToString) -> Self {
        Self {
            data: data.to_string(),
            read: false,
        }
    }
}

impl Input for Buffer {
    fn next_chunk(&mut self) -> Option<&str> {
        if self.read {
            None
        } else {
            self.read = true;
            Some(&self.data)
        }
    }
}

#[cfg(test)]
mod tests {
    // todo test self.read

    #[test]
    fn asdf() {
        todo!("nyi")
    }
}
