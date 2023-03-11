use anyhow::Result;

pub use dbg::Dbg;

use crate::model::Model;
use crate::output::Output;

mod dbg;

pub trait Generator {
    fn generate<O: Output>(&self, model: &Model, output: O) -> Result<()>;
}
