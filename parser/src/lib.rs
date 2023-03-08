mod delimited;

pub use delimited::Delimited;

use anyhow::Result;
use model::Model;
use stream::Input;

pub trait Parser {
    fn parse<T: Input>(&self, input: &T) -> Result<Model>;
}
