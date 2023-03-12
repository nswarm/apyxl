use anyhow::Result;

use crate::generator::Generator;
use crate::model::Model;
use crate::output::Output;

/// A generator that writes out the model in a readable format.
#[derive(Default)]
pub struct Dbg {}

impl Generator for Dbg {
    fn generate(&self, model: &Model, output: &mut dyn Output) -> Result<()> {
        output.write(&format!("{:#?}\n", model))
    }
}
