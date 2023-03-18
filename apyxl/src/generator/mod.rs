use anyhow::Result;

pub use dbg::Dbg;
pub use rust::Rust;

use crate::model::Api;
use crate::output::Output;

mod dbg;
mod indent;
mod rust;

pub trait Generator {
    fn generate(&mut self, api: &Api, output: &mut dyn Output) -> Result<()>;
}
