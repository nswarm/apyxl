use anyhow::Result;

pub use delimited::Delimited;

use crate::input::Input;
use crate::model::Model;

mod delimited;

pub trait Parser {
    fn parse(&self, input: &dyn Input) -> Result<Model>;
}
