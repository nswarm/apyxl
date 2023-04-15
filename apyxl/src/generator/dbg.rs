use anyhow::Result;

use crate::generator::Generator;
use crate::output::Output;
use crate::view;

/// A generator that writes out the model in a the rust [std::fmt::Debug] format.
/// Note that this format is pretty verbose.
#[derive(Default)]
pub struct Dbg {}

impl Generator for Dbg {
    fn generate<O: Output>(&mut self, model: view::Model, output: &mut O) -> Result<()> {
        output.write_str(&format!("{:#?}\n", model))
    }
}