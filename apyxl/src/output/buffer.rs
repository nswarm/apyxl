use crate::model::chunk::Chunk;
use crate::output::Output;
use anyhow::Result;

#[derive(Debug, Default)]
pub struct Buffer {
    data: String,
}

impl Buffer {
    pub fn data(&self) -> &str {
        &self.data
    }
}

impl ToString for Buffer {
    fn to_string(&self) -> String {
        self.data.clone()
    }
}

impl Output for Buffer {
    fn write_chunk(&mut self, _: &Chunk) -> Result<()> {
        // Buffer does nothing with chunks.
        Ok(())
    }

    fn write(&mut self, data: &str) -> Result<()> {
        self.data.push_str(data);
        Ok(())
    }

    fn write_char(&mut self, data: char) -> Result<()> {
        self.data.push(data);
        Ok(())
    }

    fn newline(&mut self) -> Result<()> {
        self.data.push('\n');
        Ok(())
    }
}

// Alternative for Buffer if actually operating on strings.
impl Output for String {
    fn write_chunk(&mut self, _: &Chunk) -> Result<()> {
        // String does nothing with chunks.
        Ok(())
    }

    fn write(&mut self, data: &str) -> Result<()> {
        self.push_str(data);
        Ok(())
    }

    fn write_char(&mut self, data: char) -> Result<()> {
        self.push(data);
        Ok(())
    }

    fn newline(&mut self) -> Result<()> {
        self.push('\n');
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::output::Buffer;
    use crate::Output;
    use anyhow::Result;

    #[test]
    fn write_str() -> Result<()> {
        let mut output = Buffer::default();
        output.write("asdf")?;
        assert_eq!(output.to_string(), "asdf");
        Ok(())
    }

    #[test]
    fn write_char() -> Result<()> {
        let mut output = Buffer::default();
        output.write_char(':')?;
        assert_eq!(output.to_string(), ":");
        Ok(())
    }

    #[test]
    fn write_appends() -> Result<()> {
        let mut output = Buffer::default();
        output.write("abc")?;
        output.write_char('d')?;
        output.write("efg")?;
        assert_eq!(output.to_string(), "abcdefg");
        Ok(())
    }
}
