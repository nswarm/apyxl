use anyhow::Result;

pub use dbg::Dbg;
pub use rust::Rust;

use crate::output::Output;
use crate::view;

mod dbg;
mod rust;

pub trait Generator {
    fn generate<O: Output>(&mut self, model: view::Model, output: &mut O) -> Result<()>;
}
