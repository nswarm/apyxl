use anyhow::{Context, Result};
use apyxl::input;
use apyxl::{generator, output, parser, Executor};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

fn main() -> Result<()> {
    env_logger::init();
    let examples_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?).join("examples");
    let fake_platform_root = examples_dir.join("fake_platform");
    let input_root = fake_platform_root.join("src");
    let file_name = PathBuf::from(file!())
        .with_extension("")
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let output_root = examples_dir.join(format!("output/{}", file_name));
    let input = input::Glob::new_with_root(&input_root, "**/*.rs")?;
    let output = output::FileSet::new(output_root)?;
    Executor::new(input, parser::Rust::default())
        .parser_config(parser_config(&fake_platform_root)?)
        .generator(generator::Rust::default())
        .output(output)
        .execute()
}

fn parser_config(dir: &Path) -> Result<parser::Config> {
    let file = File::open(dir.join("parser_config.json")).context("read parser config")?;
    let reader = BufReader::new(file);
    Ok(serde_json::from_reader(reader)?)
}
