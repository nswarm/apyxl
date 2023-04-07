use anyhow::Result;
use apyxl::input;
use apyxl::{generator, output, parser, Executor};
use std::path::PathBuf;

fn main() -> Result<()> {
    let project_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let mut input = input::FileSet::new(
        &project_dir,
        &[
            project_dir.join("examples/simple_input/dto.rs"),
            project_dir.join("examples/simple_input/rpc.rs"),
            project_dir.join("examples/simple_input/namespace.rs"),
        ],
    )?;
    Executor::default()
        .input(&mut input)
        .parser(&parser::Rust::default())
        .generator(
            &mut generator::Dbg::default(),
            vec![&mut output::StdOut::default()],
        )
        .execute()
}
