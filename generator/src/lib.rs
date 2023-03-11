use anyhow::Result;
use model::Model;
use output::Output;

pub trait Generator {
    fn generate<O: Output>(&self, model: &Model) -> Result<O>;
}
