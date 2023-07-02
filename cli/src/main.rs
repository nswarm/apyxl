use std::cell::RefCell;
use std::fs::File;
use std::io::BufReader;
use std::rc::Rc;

use anyhow::{Context, Result};
use clap::Parser;

use crate::config::{Config, GeneratorName, Output};

mod config;

fn main() -> Result<()> {
    env_logger::init();
    let config = Config::parse();
    let input = apyxl::input::Glob::new(&config.input)?;
    let parser = parser(&config);
    let parser_config = parser_config(&config)?;
    let mut outputs = Vec::<Rc<RefCell<dyn apyxl::Output>>>::new();
    let mut exe = apyxl::Executor::new(input, parser);
    if let Some(parser_config) = parser_config {
        exe = exe.parser_config(parser_config);
    }
    for generator_name in &config.generator {
        exe = add_generator(*generator_name, &config, exe, &mut outputs)?;
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
    outputs: &mut Vec<Rc<RefCell<dyn apyxl::Output>>>,
) -> Result<apyxl::Executor<I, P>> {
    exe = exe.generator(generator_name.create_impl());
    for output_config in &config.output {
        if output_config.generator == generator_name {
            let output = output(config, output_config)?;
            outputs.push(output.clone());
            exe = exe.output_ptr(output)
        }
    }
    Ok(exe)
}

fn output(config: &Config, output: &Output) -> Result<Rc<RefCell<apyxl::output::FileSet>>> {
    Ok(Rc::new(RefCell::new(apyxl::output::FileSet::new(
        config.output_root.join(&output.path),
    )?)))
}
