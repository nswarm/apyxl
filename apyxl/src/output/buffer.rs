use crate::model::chunk::Chunk;
use crate::output::Output;
use anyhow::Result;

#[derive(Debug, Default)]
pub struct Buffer {
    data: String,
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

    fn write_str(&mut self, data: &str) -> Result<()> {
        self.data.push_str(data);
        Ok(())
    }

    fn write(&mut self, data: char) -> Result<()> {
        self.data.push(data);
        Ok(())
    }

    fn newline(&mut self) -> Result<()> {
        self.data.push('\n');
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
        output.write_str("asdf")?;
        assert_eq!(output.to_string(), "asdf");
        Ok(())
    }

    #[test]
    fn write_char() -> Result<()> {
        let mut output = Buffer::default();
        output.write(':')?;
        assert_eq!(output.to_string(), ":");
        Ok(())
    }

    #[test]
    fn write_appends() -> Result<()> {
        let mut output = Buffer::default();
        output.write_str("abc")?;
        output.write('d')?;
        output.write_str("efg")?;
        assert_eq!(output.to_string(), "abcdefg");
        Ok(())
    }
}
