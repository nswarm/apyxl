use anyhow::Result;

use crate::input::Input;

mod rust;

use crate::model::api;
pub use rust::Rust;

pub trait Parser {
    fn parse<'a, I: Input + 'a>(&self, input: &'a mut I) -> Result<api::Builder<'a>>;
}
