use anyhow::Result;
use std::fmt::Debug;

pub use dbg::Dbg;
pub use rust::Rust;
pub use markdown::Markdown;

use crate::output::Output;
use crate::view;

mod dbg;
mod markdown;
mod rust;
mod util;

pub trait Generator: Debug {
    fn generate(&mut self, model: view::Model, output: &mut dyn Output) -> Result<()>;
}

impl Generator for Box<dyn Generator> {
    fn generate(&mut self, model: view::Model, output: &mut dyn Output) -> Result<()> {
        (**self).generate(model, output)
    }
}
