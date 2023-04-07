use anyhow::Result;

pub use dbg::Dbg;
pub use rust::Rust;

use crate::model::Model;
use crate::output::Output;

mod dbg;
mod rust;

pub trait Generator {
    fn generate<O: Output>(&mut self, model: &Model, output: &mut O) -> Result<()>;
}
