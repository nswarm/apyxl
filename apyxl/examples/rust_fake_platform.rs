use anyhow::Result;
use apyxl::input;
use apyxl::{generator, output, parser, Executor};
use std::path::PathBuf;

fn main() -> Result<()> {
    env_logger::init();
    let examples_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?).join("examples");
    let input_root = examples_dir.join("fake_platform/src");
    let file_name = PathBuf::from(file!())
        .with_extension("")
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let output_root = examples_dir.join(format!("output/{}", file_name));
    let mut input = input::Glob::new(&input_root, "**/*.rs")?;
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
