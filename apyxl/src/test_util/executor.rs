use crate::model::{Builder, Metadata};
use crate::{input, model, parser, Parser};
use lazy_static::lazy_static;

#[derive(Default)]
pub struct TestExecutor {
    input: input::Buffer,
    parser: parser::Rust,
}

lazy_static! {
    static ref CONFIG: parser::Config = parser::Config::default();
}

impl TestExecutor {
    pub fn new<S: ToString>(data: S) -> Self {
        Self {
            input: input::Buffer::new(data),
            parser: parser::Rust::default(),
        }
    }

    pub fn api(&mut self) -> model::Api {
        let mut builder = Builder::default();
        self.parser
            .parse(&CONFIG, &mut self.input, &mut builder)
            .expect("failed to parse input");
        builder.into_api()
    }

    pub fn model(&mut self) -> model::Model {
        // Skip deps which rely on valid api.
        model::Model::without_deps(self.api(), Metadata::default())
    }

    pub fn build(&mut self) -> model::Model {
        let mut builder = Builder::default();
        self.parser
            .parse(&CONFIG, &mut self.input, &mut builder)
            .expect("failed to parse input");
        builder.build().unwrap_or_else(|errs| {
            for err in errs {
                println!("Error: {}", err)
            }
            panic!("^ Validation errors building api ^");
        })
    }
}
