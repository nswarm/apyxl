use anyhow::{anyhow, Result};
use itertools::Itertools;

use crate::generator::Generator;
use crate::input::Input;
use crate::model::ValidationError;
use crate::output::Output;
use crate::parser::Parser;

#[derive(Default)]
pub struct Executor<'a, I: Input, P: Parser, G: Generator, O: Output> {
    input: Option<&'a mut I>,
    parser: Option<&'a P>,
    generator_infos: Vec<GeneratorInfo<'a, G, O>>,
}

pub struct GeneratorInfo<'a, G: Generator, O: Output> {
    generator: &'a mut G,
    outputs: Vec<&'a mut O>,
}

impl<'a, I: Input, P: Parser, G: Generator, O: Output> Executor<'a, I, P, G, O> {
    pub fn input(mut self, input: &'a mut I) -> Self {
        self.input = Some(input);
        self
    }

    pub fn parser(mut self, parser: &'a P) -> Self {
        self.parser = Some(parser);
        self
    }

    pub fn generator(mut self, generator: &'a mut G, outputs: Vec<&'a mut O>) -> Self {
        self.generator_infos
            .push(GeneratorInfo { generator, outputs });
        self
    }

    pub fn execute(self) -> Result<()> {
        let input = self
            .input
            .ok_or_else(|| anyhow!("no 'input' has been specified"))?;
        let parser = self
            .parser
            .ok_or_else(|| anyhow!("no 'parser' has been specified"))?;

        if self.generator_infos.is_empty() {
            return Err(anyhow!("no 'generators' have been specified"));
        }
        for info in &self.generator_infos {
            if info.outputs.is_empty() {
                return Err(anyhow!(
                    "each 'generator' have at least one 'output' specified"
                ));
            }
        }

        let model_builder = parser.parse(input)?;
        let model = match model_builder.build() {
            Ok(model) => model,
            Err(errors) => {
                return Err(anyhow!(
                    "API validation failed.\n{}",
                    errors_to_string(&errors)
                ))
            }
        };

        for info in self.generator_infos {
            for output in info.outputs {
                // todo log generating for abc to output xyz
                info.generator.generate(model.view(), output)?;
            }
        }
        Ok(())
    }
}

fn errors_to_string(errors: &[ValidationError<'_>]) -> String {
    errors.iter().map(|e| format!("{}", e)).join("\n")
}

#[cfg(test)]
mod tests {
    use anyhow::{anyhow, Result};

    use crate::generator::Generator;
    use crate::input::Input;
    use crate::model::{Api, Dto, NamespaceChild, UNDEFINED_NAMESPACE};
    use crate::output::Output;
    use crate::parser::Parser;
    use crate::{model, view};

    mod execute {
        use anyhow::Result;

        use crate::executor::tests::{FakeGenerator, FakeParser};
        use crate::{input, output, Executor};

        #[test]
        fn happy_path() -> Result<()> {
            let mut output = output::Buffer::default();
            let parser = FakeParser::default();
            Executor::default()
                .input(&mut input::Buffer::new(parser.test_data(1)))
                .parser(&parser)
                .generator(&mut FakeGenerator::default(), vec![&mut output])
                .execute()?;
            assert_eq!(output.to_string(), parser.test_data(1));
            Ok(())
        }

        #[test]
        fn calls_all_generators_with_correct_outputs() -> Result<()> {
            let input_vec = vec![1, 2, 3];
            let parser = FakeParser::new(",");
            let mut gen0 = FakeGenerator::new("/");
            let mut gen1 = FakeGenerator::new(":");
            let mut output0 = output::Buffer::default();
            let mut output1 = output::Buffer::default();
            let mut output2 = output::Buffer::default();
            Executor::default()
                .input(&mut input::Buffer::new(parser.test_data_vec(&input_vec)))
                .parser(&parser)
                .generator(&mut gen0, vec![&mut output0])
                .generator(&mut gen1, vec![&mut output1, &mut output2])
                .execute()?;
            assert_eq!(output0.to_string(), gen0.expected(&input_vec));
            assert_eq!(output1.to_string(), gen1.expected(&input_vec));
            assert_eq!(output2.to_string(), gen1.expected(&input_vec));
            Ok(())
        }
    }

    mod validation {
        use crate::executor::tests::{FakeGenerator, FakeParser};
        use crate::executor::Executor;
        use crate::{input, output};

        #[test]
        fn missing_input() {
            let result =
                Executor::<input::Buffer, FakeParser, FakeGenerator, output::Buffer>::default()
                    // no input
                    .parser(&FakeParser::default())
                    .generator(
                        &mut FakeGenerator::default(),
                        vec![&mut output::Buffer::default()],
                    )
                    .execute();
            assert!(result.is_err())
        }

        #[test]
        fn missing_parser() {
            let parser = FakeParser::default();
            let result =
                Executor::<input::Buffer, FakeParser, FakeGenerator, output::Buffer>::default()
                    .input(&mut input::Buffer::new(parser.test_data(1)))
                    // no parser
                    .generator(
                        &mut FakeGenerator::default(),
                        vec![&mut output::Buffer::default()],
                    )
                    .execute();
            assert!(result.is_err())
        }

        #[test]
        fn missing_generator() {
            let parser = FakeParser::default();
            let result =
                Executor::<input::Buffer, FakeParser, FakeGenerator, output::Buffer>::default()
                    .input(&mut input::Buffer::new(parser.test_data(1)))
                    .parser(&parser)
                    // no generator
                    .execute();
            assert!(result.is_err())
        }

        #[test]
        fn missing_output() {
            let parser = FakeParser::default();
            let result =
                Executor::<input::Buffer, FakeParser, FakeGenerator, output::Buffer>::default()
                    .input(&mut input::Buffer::new(parser.test_data(1)))
                    .parser(&FakeParser::default())
                    .generator(
                        &mut FakeGenerator::default(),
                        vec![
                            /* no outputs */
                        ],
                    )
                    .execute();
            assert!(result.is_err())
        }
    }

    #[derive(Default)]
    struct FakeParser {
        delimiter: String,
    }
    impl FakeParser {
        pub fn new(delimiter: impl ToString) -> Self {
            Self {
                delimiter: delimiter.to_string(),
            }
        }

        fn test_data(&self, i: i32) -> String {
            self.test_data_vec(&vec![i])
        }

        fn test_data_vec(&self, v: &Vec<i32>) -> String {
            let mut data = String::new();
            for i in v {
                data.push_str(&i.to_string());
                if *i < v.len() as i32 {
                    data.push_str(&self.delimiter);
                }
            }
            data
        }
    }
    impl Parser for FakeParser {
        fn parse<'a, I: Input + 'a>(&self, input: &'a mut I) -> Result<model::Builder<'a>> {
            let mut builder = model::Builder::default();
            builder.merge(Api {
                name: UNDEFINED_NAMESPACE,
                children: input
                    .next_chunk()
                    .ok_or_else(|| anyhow!("no input data!"))?
                    .1 // data
                    .split(&self.delimiter)
                    .filter_map(|name| {
                        if name.is_empty() {
                            None
                        } else {
                            Some(Dto {
                                name,
                                ..Default::default()
                            })
                        }
                    })
                    .map(NamespaceChild::Dto)
                    .collect::<Vec<NamespaceChild>>(),
                attributes: Default::default(),
            });
            Ok(builder)
        }
    }

    #[derive(Default)]
    struct FakeGenerator {
        delimiter: String,
    }

    impl FakeGenerator {
        pub fn new(delimiter: impl ToString) -> Self {
            Self {
                delimiter: delimiter.to_string(),
            }
        }

        pub fn expected(&self, v: &[i32]) -> String {
            v.iter()
                .map(|x| x.to_string())
                .collect::<Vec<String>>()
                .join(&self.delimiter)
        }
    }

    impl Generator for FakeGenerator {
        fn generate<O: Output>(&mut self, model: view::Model, output: &mut O) -> Result<()> {
            let dto_names = model
                .api()
                .dtos()
                .map(|dto| dto.name().to_string())
                .collect::<Vec<String>>();
            output.write_str(&dto_names.join(&self.delimiter))?;
            Ok(())
        }
    }
}
