use anyhow::Result;
use apyxl::input;
use apyxl::{generator, output, parser, Executor};
use std::path::PathBuf;

fn main() -> Result<()> {
    env_logger::init();
    let examples_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?).join("examples");
    let input_root = examples_dir.join("simple_input");
    let file_name = PathBuf::from(file!())
        .with_extension("")
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let output_root = examples_dir.join(format!("output/{}", file_name));
    let input = input::FileSet::new(&input_root, &["dto.rs", "rpc.rs", "namespace.rs"])?;
    let output = output::FileSet::new(output_root)?;
    Executor::new(input, parser::Rust::default())
        .generator(generator::Rust::default())
        .output(output::StdOut::default())
        .output(output)
        .execute()
}
