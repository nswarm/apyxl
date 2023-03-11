use anyhow::Result;

pub use delimited::Delimited;

use crate::input::Input;
use crate::model::Model;

mod delimited;

pub trait Parser {
    fn parse<T: Input>(&self, input: &T) -> Result<Model>;
}
