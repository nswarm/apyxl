use anyhow::Result;
use std::fmt::Debug;

pub use dbg::Dbg;
pub use rust::Rust;

use crate::output::Output;
use crate::view;

mod dbg;
mod rust;

pub trait Generator: Debug {
    fn generate(&mut self, model: view::Model, output: &mut dyn Output) -> Result<()>;
}

impl Generator for Box<dyn Generator> {
    fn generate(&mut self, model: view::Model, output: &mut dyn Output) -> Result<()> {
        (**self).generate(model, output)
    }
}
