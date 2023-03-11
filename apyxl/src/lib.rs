use anyhow::{anyhow, Result};

use crate::generator::Generator;
use crate::input::Input;
use crate::output::Output;
use crate::parser::Parser;

mod generator;
mod input;
mod model;
mod output;
mod parser;

#[derive(Default)]
pub struct Executor {
    input: Option<Box<dyn Input>>,
    parser: Option<Box<dyn Parser>>,
    generator_infos: Vec<GeneratorInfo>,
}

pub struct GeneratorInfo {
    generator: Box<dyn Generator>,
    outputs: Vec<Box<dyn Output>>,
}

impl Executor {
    pub fn input<I: Input + 'static>(mut self, input: I) -> Self {
        self.input = Some(Box::new(input));
        self
    }

    pub fn parser<P: Parser + 'static>(mut self, parser: P) -> Self {
        self.parser = Some(Box::new(parser));
        self
    }

    pub fn generator<G: Generator + 'static>(
        mut self,
        generator: G,
        outputs: Vec<Box<dyn Output>>,
    ) -> Self {
        self.generator_infos.push(GeneratorInfo {
            generator: Box::new(generator),
            outputs,
        });
        self
    }

    pub fn execute(self) -> Result<()> {
        let input = self
            .input
            .ok_or_else(|| anyhow!("no 'input' has been specified"))?;
        let parser = self
            .parser
            .ok_or_else(|| anyhow!("no 'parser' has been specified"))?;

        // validate generators and outputs

        // todo log parsing
        let model = parser.parse(&*input)?;

        for info in self.generator_infos {
            for mut output in info.outputs {
                // todo log generating for abc to output xyz
                info.generator.generate(&model, &mut *output)?;
            }
        }
        Ok(())
    }
}

pub fn execute() -> Result<()> {
    let input = input::Buffer::new("abc,def,ghi");
    let output = output::StdOut::new("--- DELIMITED ---");

    Executor::default()
        .input(input)
        .parser(parser::Delimited::new(","))
        .generator(
            generator::Dbg::default(),
            vec![
                Box::new(output::StdOut::new("Debug Model:")),
                Box::new(output::StdOut::new("Debug Model #2:")),
            ],
        )
        .execute()
}
