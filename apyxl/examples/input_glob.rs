use anyhow::Result;
use apyxl::input;
use apyxl::{generator, output, parser, Executor};
use std::path::PathBuf;

fn main() -> Result<()> {
    let project_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let mut input = input::Glob::new(project_dir.join("examples/simple_input"), "**/*.rs")?;
    Executor::default()
        .input(&mut input)
        .parser(&parser::Rust::default())
        .generator(
            &mut generator::Dbg::default(),
            vec![&mut output::StdOut::default()],
        )
        .execute()
}
