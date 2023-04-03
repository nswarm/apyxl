use crate::Output;
use anyhow::Result;
use log::error;

/// Indented wraps an existing output and keeps track of current indentation level.
///
/// After each [Output::newline], a "pending" newline will be held until the next [Output::write]
/// call at which point the indentation will be applied before the new characters. This allows more
/// intuitive usage of [Indented::indent], in particular: the order in which you call [Output::newline]
/// and [Indented::indent] does not matter.
pub struct Indented<'a, O: Output> {
    depth: u32,
    has_pending_indent: bool,
    indent: &'a str,
    output: &'a mut O,
}

impl<'a, O: Output> Indented<'_, O> {
    pub fn new(output: &'a mut O, indent: &'a str) -> Indented<'a, O> {
        Indented {
            depth: 0,
            // Start true in case indent is modified before the first write, it would be expected
            // that it applies to that write.
            has_pending_indent: true,
            indent,
            output,
        }
    }

    /// Current indentation depth.
    pub fn depth(&self) -> u32 {
        self.depth
    }

    /// Adds [amount] to the indent depth.
    pub fn indent(&mut self, amount: i32) {
        match amount {
            amount if amount > 0 => self.add(amount as u32),
            amount if amount < 0 => self.sub(-amount as u32),
            _ => {}
        }
    }

    fn add(&mut self, rhs: u32) {
        if self.depth.checked_add(rhs).is_none() {
            error!("reached maximum indent level! ({})", u32::MAX);
        }
        self.depth = self.depth.saturating_add(rhs);
    }

    fn sub(&mut self, rhs: u32) {
        if self.depth.checked_sub(rhs).is_none() {
            error!("cannot decrement indent below 0! mismatched inc/dec?");
        }
        self.depth = self.depth.saturating_sub(rhs);
    }

    fn write_pending_indent(&mut self) -> Result<()> {
        if !self.has_pending_indent {
            return Ok(());
        }
        self.has_pending_indent = false;
        for _ in 0..self.depth {
            self.output.write_str(self.indent)?;
        }
        Ok(())
    }
}

impl<O: Output> Output for Indented<'_, O> {
    fn write_str(&mut self, data: &str) -> Result<()> {
        self.write_pending_indent()?;
        self.output.write_str(data)
    }

    fn write(&mut self, data: char) -> Result<()> {
        self.write_pending_indent()?;
        self.output.write(data)
    }

    fn newline(&mut self) -> Result<()> {
        self.output.newline()?;
        self.has_pending_indent = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::output::{Buffer, Indented};
    use crate::Output;
    use anyhow::Result;

    #[test]
    fn add_sub_depth() {
        let mut output = Buffer::default();
        let mut indent = Indented::new(&mut output, "  ");
        indent.indent(1);
        assert_eq!(indent.depth(), 1);
        indent.indent(2);
        assert_eq!(indent.depth(), 3);
        indent.indent(-1);
        assert_eq!(indent.depth(), 2);
        indent.indent(-2);
        assert_eq!(indent.depth(), 0);
    }

    #[test]
    fn sub_does_not_go_below_0() {
        let mut output = Buffer::default();
        let mut indent = Indented::new(&mut output, "  ");
        indent.indent(2);
        indent.indent(-99);
        assert_eq!(indent.depth(), 0);
    }

    #[test]
    fn write_applies_indent_if_pending() -> Result<()> {
        let mut output = Buffer::default();
        let mut indent = Indented::new(&mut output, "  ");
        indent.indent(2);
        indent.write('x')?;
        assert_eq!(output.to_string(), "    x");
        Ok(())
    }

    #[test]
    fn write_str_applies_indent_if_pending() -> Result<()> {
        let mut output = Buffer::default();
        let mut indent = Indented::new(&mut output, "  ");
        indent.indent(2);
        indent.write_str("xx")?;
        assert_eq!(output.to_string(), "    xx");
        Ok(())
    }

    #[test]
    fn clears_pending_indent() -> Result<()> {
        let mut output = Buffer::default();
        let mut indent = Indented::new(&mut output, "  ");
        indent.indent(2);
        indent.write_str("x")?;
        // If indent is _not_ reset, this will apply indent again.
        indent.write_str("x")?;
        assert_eq!(output.to_string(), "    xx");
        Ok(())
    }

    #[test]
    fn indent_before_newline() -> Result<()> {
        let mut output = Buffer::default();
        let mut indent = Indented::new(&mut output, "  ");
        indent.indent(2);
        indent.newline()?;
        indent.write('x')?;
        assert_eq!(output.to_string(), "\n    x");
        Ok(())
    }

    #[test]
    fn indent_after_newline() -> Result<()> {
        let mut output = Buffer::default();
        let mut indent = Indented::new(&mut output, "  ");
        indent.newline()?;
        indent.indent(2);
        indent.write('x')?;
        assert_eq!(output.to_string(), "\n    x");
        Ok(())
    }
}
