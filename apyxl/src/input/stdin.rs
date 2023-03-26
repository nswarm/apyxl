use std::io::{stdin, Read};

use crate::input::Input;

#[derive(Default)]
pub struct StdIn {
    data: Vec<String>,
}

impl Input for StdIn {
    fn next_chunk(&mut self) -> Option<&str> {
        self.data.push(String::new());
        match stdin().read_to_string(self.data.last_mut().unwrap()) {
            Ok(len) if len > 0 => Some(self.data.last().unwrap()),
            _ => None,
        }
    }
}
