use crate::output::Output;
use anyhow::Result;

#[derive(Default)]
pub struct Buffer {
    data: String,
}

impl ToString for Buffer {
    fn to_string(&self) -> String {
        self.data.clone()
    }
}

impl Output for Buffer {
    fn write(&mut self, data: &str) -> Result<()> {
        self.data.push_str(data);
        Ok(())
    }
}
