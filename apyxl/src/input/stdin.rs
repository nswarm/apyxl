use std::io::{stdin, Read};

use anyhow::Result;

use crate::input::Input;

pub struct StdIn {
    data: String,
}

impl StdIn {
    pub fn new() -> Result<Self> {
        let mut data = String::new();
        let _ = stdin().read_to_string(&mut data)?;
        Ok(Self { data })
    }
}

impl Input for StdIn {
    fn data(&self) -> &str {
        &self.data
    }
}
