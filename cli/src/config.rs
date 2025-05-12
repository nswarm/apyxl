use anyhow::{anyhow, Result};
use apyxl::model::Builder;
use apyxl::Input;
use clap::{Parser, ValueEnum};
use itertools::Itertools;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "apyxl", author, version, about)]
pub struct Config {
    /// Unix-style glob of files to be parsed as API source files.
    ///
    /// If running in a unix-style shell, you'll need to enclose inside '' or it will be expanded
    /// by the shell itself.
    ///
    /// If the glob is relative, it will be relative to the current working directory.
    #[arg(short, long, value_name = "GLOB")]
    pub input: String,

    /// Name of the parser to use.
    #[arg(short, long)]
    pub parser: ParserName,

    /// Path to a [apyxl::parser::Config] in json format.
    #[arg(long)]
    pub parser_config: Option<PathBuf>,

    /// Name of generators to use.
    #[arg(short, long, required(true))]
    pub generator: Vec<GeneratorName>,

    /// All relative --outputs will be relative to this path. Defaults to working directory.
    #[arg(long, default_value = ".")]
    pub output_root: PathBuf,

    /// Each argument should be a key=value pair where the key is a [GeneratorName] and the value
    /// is path to an empty (or nonexistent) directory.
    ///
    /// See also --output-root to set the relative root directory.
    ///
    /// If not supplied, the name of the generator is used as the directory name.
    ///
    /// Example:
    ///     --output-root ./root/dir -o rust=rrr cpp=ccc
    /// would result in a file structure like
    ///     ./root/dir/rrr (generated rust files)
    ///     ./root/dir/ccc (generated cpp files)
    #[arg(short, long, value_parser=parse_output)]
    pub output: Vec<Output>,

    /// Every generator name in the list will generate to stdout in addition to other outputs.
    #[arg(long)]
    pub stdout: Vec<GeneratorName>,

    /// All generators will use stdout output only.
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(ValueEnum, Copy, Clone, Debug)]
pub enum ParserName {
    #[clap(name = "rust")]
    Rust,
    #[clap(name = "csharp")]
    CSharp,
}

pub enum ParserImpl {
    Rust(apyxl::parser::Rust),
    CSharp(csharp::Parser),
}

#[derive(ValueEnum, Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum GeneratorName {
    Rust,
    Markdown,
}

#[derive(Clone, Debug)]
pub struct Output {
    pub generator: GeneratorName,
    pub path: PathBuf,
}

fn parse_output(arg: &str) -> Result<Output> {
    let vec = arg.split('=').collect_vec();
    if vec.len() != 2 {
        return Err(anyhow!(
            "output must be in the form '<generator>=<output/path>'"
        ));
    }
    let generator = GeneratorName::from_str(vec[0], true)
        .map_err(|_| anyhow!("'{}' is not a valid generator name", vec[0]))?;
    let path = PathBuf::from(vec[1]);
    Ok(Output { generator, path })
}

impl ParserName {
    pub fn create_impl(&self) -> ParserImpl {
        match self {
            ParserName::Rust => ParserImpl::Rust(apyxl::parser::Rust::default()),
            ParserName::CSharp => ParserImpl::CSharp(csharp::Parser::default()),
        }
    }
}

impl apyxl::Parser for ParserImpl {
    fn parse<'a, I: Input + 'a>(
        &self,
        config: &'a apyxl::parser::Config,
        input: &'a mut I,
        builder: &mut Builder<'a>,
    ) -> Result<()> {
        match self {
            ParserImpl::Rust(p) => p.parse(config, input, builder),
            ParserImpl::CSharp(p) => p.parse(config, input, builder),
        }
    }
}

impl GeneratorName {
    pub fn create_impl(&self) -> Box<dyn apyxl::Generator> {
        match self {
            GeneratorName::Rust => Box::new(apyxl::generator::Rust::default()),
            GeneratorName::Markdown => Box::new(apyxl::generator::Markdown::default()),
        }
    }
}
