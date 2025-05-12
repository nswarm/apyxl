use crate::input::Input;
use crate::model;
use anyhow::Result;
pub use config::*;
pub use rust::Rust;

pub mod comment;
pub mod error;
pub mod test_util;
pub mod util;

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
