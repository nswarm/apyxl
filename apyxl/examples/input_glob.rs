use anyhow::Result;
use apyxl::input;
use apyxl::{generator, output, parser, Executor};
use std::path::PathBuf;

fn main() -> Result<()> {
    env_logger::init();
    let project_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let root = project_dir.join("examples/simple_input");
    let input = input::Glob::new_with_root(&root, "**/*.rs")?;
    Executor::new(input, parser::Rust::default())
        .generator(generator::Dbg::default())
        .output(output::StdOut::default())
        .execute()
}
