use anyhow::Result;
use apyxl::input;
use apyxl::{generator, output, parser, Executor};
use std::path::PathBuf;

fn main() -> Result<()> {
    env_logger::init();
    let examples_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?).join("examples");
    let input_root = examples_dir.join("simple_input");
    let output_root = examples_dir.join("output/in_out_file_set");
    let mut input = input::FileSet::new(&input_root, &["dto.rs", "rpc.rs", "namespace.rs"])?;
    let mut output = output::FileSet::new(output_root)?;
    Executor::default()
        .input(&mut input)
        .parser(&parser::Rust::default())
        .generator(
            &mut generator::Rust::default(),
            vec![&mut output::StdOut::default(), &mut output],
        )
        .execute()
}
