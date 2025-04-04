use std::cell::RefCell;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::rc::Rc;

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};

use crate::config::{Config, GeneratorName, Output};

mod config;

fn main() -> Result<()> {
    env_logger::init();
    let config = Config::parse();
    let input = apyxl::input::Glob::new(&config.input)?;
    let parser = parser(&config);
    let parser_config = parser_config(&config)?;
    let mut exe = apyxl::Executor::new(input, parser);
    if let Some(parser_config) = parser_config {
        exe = exe.parser_config(parser_config);
    }
    for generator_name in &config.generator {
        exe = add_generator(*generator_name, &config, exe)?;
    }
    exe.execute()
}

fn parser(config: &Config) -> impl apyxl::Parser {
    config.parser.create_impl()
}

fn parser_config(config: &Config) -> Result<Option<apyxl::parser::Config>> {
    match &config.parser_config {
        None => Ok(None),
        Some(path) => {
            let file = File::open(path).context("read parser config")?;
            let reader = BufReader::new(file);
            Ok(Some(serde_json::from_reader(reader)?))
        }
    }
}

fn add_generator<I: apyxl::Input, P: apyxl::Parser>(
    generator_name: GeneratorName,
    config: &Config,
    mut exe: apyxl::Executor<I, P>,
) -> Result<apyxl::Executor<I, P>> {
    exe = exe.generator(generator_name.create_impl());
    if config.dry_run {
        return Ok(exe.output(apyxl::output::StdOut::default()));
    }
    for output_config in &config.output {
        if output_config.generator != generator_name {
            continue;
        }
        let path = output_path(config, output_config);
        let output = output(path)?;
        exe = exe.output_ptr(output);
    }
    if exe.output_count() == 0 {
        let path = default_output_path(config, generator_name);
        let output = output(path)?;
        exe = exe.output_ptr(output);
    }
    if config.stdout.contains(&generator_name) {
        exe = exe.output(apyxl::output::StdOut::default());
    }
    Ok(exe)
}

fn default_output_path(config: &Config, generator_name: GeneratorName) -> PathBuf {
    config
        .output_root
        .join(generator_name.to_possible_value().unwrap().get_name())
}

fn output_path(config: &Config, output: &Output) -> PathBuf {
    config.output_root.join(&output.path)
}

fn output(path: PathBuf) -> Result<Rc<RefCell<apyxl::output::FileSet>>> {
    Ok(Rc::new(RefCell::new(apyxl::output::FileSet::new(path)?)))
}

#[cfg(test)]
mod tests {
    use crate::config::{Config, GeneratorName, Output, ParserName};
    use crate::{default_output_path, output_path};
    use std::path::PathBuf;

    #[test]
    fn test_output_path() {
        let mut config = test_config();
        config.output_root = PathBuf::from("a/b/c");
        assert_eq!(
            output_path(
                &config,
                &Output {
                    generator: GeneratorName::Rust,
                    path: PathBuf::from("x/y/z"),
                }
            ),
            PathBuf::from("a/b/c/x/y/z")
        );
    }

    #[test]
    fn test_default_output_path() {
        let mut config = test_config();
        config.output_root = PathBuf::from("a/b/c");
        assert_eq!(
            default_output_path(&config, GeneratorName::Rust),
            PathBuf::from("a/b/c/rust")
        );
    }

    fn test_config() -> Config {
        Config {
            input: "".to_string(),
            parser: ParserName::Rust,
            parser_config: None,
            generator: vec![],
            output_root: Default::default(),
            output: vec![],
            stdout: vec![],
            dry_run: false,
        }
    }
}
