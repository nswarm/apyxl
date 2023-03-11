use crate::Output;
use anyhow::Result;
use std::io::{stdout, Write};

#[derive(Default)]
pub struct StdOut {}

impl StdOut {}

impl Output for StdOut {
    fn write(&self, data: &str) -> Result<()> {
        let _ = stdout().write(data.as_bytes())?;
        Ok(())
    }
}
