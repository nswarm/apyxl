use log::error;

#[derive(Default)]
pub struct Indent {
    value: u32,
}

impl Indent {
    pub fn value(&self) -> u32 {
        self.value
    }

    pub fn add(&mut self, rhs: u32) {
        if self.value.checked_add(rhs).is_none() {
            error!("reached maximum indent level! ({})", u32::MAX);
        }
        self.value = self.value.saturating_add(rhs);
    }

    pub fn sub(&mut self, rhs: u32) {
        if self.value.checked_sub(rhs).is_none() {
            error!("cannot decrement indent below 0! mismatched inc/dec?");
        }
        self.value = self.value.saturating_sub(rhs);
    }
}

#[cfg(test)]
mod test {
    use crate::generator::indent::Indent;

    #[test]
    fn inc_dec() {
        let mut indent = Indent::default();
        indent.add(1);
        assert_eq!(indent.value(), 1);
        indent.add(2);
        assert_eq!(indent.value(), 3);
        indent.sub(1);
        assert_eq!(indent.value(), 2);
        indent.sub(2);
        assert_eq!(indent.value(), 0);
    }

    #[test]
    fn dec_does_not_go_below_0() {
        let mut indent = Indent::default();
        indent.add(2);
        indent.sub(99);
        assert_eq!(indent.value(), 0);
    }
}
