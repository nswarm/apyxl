use anyhow::Result;

use crate::input::Input;
use crate::model::Api;

mod rust;

pub trait Parser {
    fn parse(&self, input: &dyn Input) -> Result<Api>;
}
