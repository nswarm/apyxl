use crate::model::Metadata;
use crate::{input, model, parser, Parser};

#[derive(Default)]
pub struct TestExecutor {
    input: input::Buffer,
    parser: parser::Rust,
}

impl TestExecutor {
    pub fn new<D: ToString>(data: D) -> Self {
        Self {
            input: input::Buffer::new(data),
            parser: parser::Rust::default(),
        }
    }

    pub fn api(&mut self) -> model::Api {
        self.parser
            .parse(&mut self.input)
            .expect("failed to parse input")
            .into_api()
    }

    pub fn model(&mut self) -> model::Model {
        model::Model::new(self.api(), Metadata::default())
    }
}
