use std::io::{stdin, Read};

use crate::input::Input;
use anyhow::Result;

pub struct StdIn {
    data: String,
}

impl StdIn {
    /// Pulls all available data from stdin immediately on creation.
    pub fn new() -> Result<Self> {
        let mut s = Self {
            data: String::new(),
        };
        stdin().read_to_string(&mut s.data)?;
        Ok(s)
    }
}

impl Input for StdIn {
    fn next_chunk(&self) -> Option<&str> {
        Some(&self.data)
    }
}
