use anyhow::Result;

pub use rust::Rust;

use crate::input::Input;
use crate::model;

mod rust;

pub trait Parser {
    fn parse<'a, I: Input + 'a>(
        &self,
        input: &'a mut I,
        builder: &mut model::Builder<'a>,
    ) -> Result<()>;
}
