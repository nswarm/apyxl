use anyhow::Result;

use crate::generator::Generator;
use crate::model::chunk;
use crate::output::Output;
use crate::view;

/// A generator that writes out the model in a the rust [std::fmt::Debug] format.
/// Note that this format is pretty verbose.
#[derive(Debug, Default)]
pub struct Dbg {}

impl Generator for Dbg {
    fn generate(&mut self, model: view::Model, output: &mut dyn Output) -> Result<()> {
        // todo how should think work w/ chunks?
        output.write_chunk(&chunk::Chunk::with_relative_file_path("dbg"))?;
        output.write_str(&format!("{:#?}\n", model))
    }
}
