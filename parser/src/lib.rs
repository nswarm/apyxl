mod delimited;

pub use delimited::Delimited;

use anyhow::Result;
use input::Input;
use model::Model;

pub trait Parser {
    fn parse<T: Input>(&self, input: &T) -> Result<Model>;
}
