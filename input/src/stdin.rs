use crate::Input;
use anyhow::Result;
use std::io::{stdin, Read};

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
