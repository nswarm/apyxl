use anyhow::Result;

pub use dbg::Dbg;

use crate::model::Api;
use crate::output::Output;

mod dbg;

pub trait Generator {
    fn generate(&self, api: &Api, output: &mut dyn Output) -> Result<()>;
}
