use anyhow::Result;

use crate::generator::Generator;
use crate::model::Model;
use crate::output::Output;

/// A generator that writes out the model in a readable format.
#[derive(Default)]
pub struct Dbg {}

impl Generator for Dbg {
    fn generate<O: Output>(&self, model: &Model, output: O) -> Result<()> {
        output.write(&format!("{:?}", model))
    }
}
