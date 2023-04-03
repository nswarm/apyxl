use anyhow::Result;

pub use dbg::Dbg;
pub use rust::Rust;

use crate::model::Api;
use crate::output::Output;

mod dbg;
mod rust;

pub trait Generator {
    fn generate<O: Output>(&mut self, api: &Api, output: &mut O) -> Result<()>;
}
