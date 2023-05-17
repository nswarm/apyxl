use anyhow::Result;

pub use config::*;
pub use rust::Rust;

use crate::input::Input;
use crate::model;

mod config;
mod rust;

pub trait Parser {
    fn parse<'a, I: Input + 'a>(
        &self,
        config: &'a Config,
        input: &'a mut I,
        builder: &mut model::Builder<'a>,
    ) -> Result<()>;
}
