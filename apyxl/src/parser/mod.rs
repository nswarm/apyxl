use anyhow::Result;

use crate::input::Input;
use crate::model::Api;

mod rust;

pub use rust::Rust;

pub trait Parser {
    fn parse<'a>(&self, input: &'a dyn Input) -> Result<Api<'a>>;
}
