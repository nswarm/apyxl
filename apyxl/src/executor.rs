use anyhow::{anyhow, Result};

use crate::generator::Generator;
use crate::input::Input;
use crate::output::Output;
use crate::parser::Parser;

#[derive(Default)]
pub struct Executor<'a> {
    input: Option<&'a mut dyn Input>,
    parser: Option<&'a dyn Parser>,
    generator_infos: Vec<GeneratorInfo<'a>>,
}

pub struct GeneratorInfo<'a> {
    generator: &'a mut dyn Generator,
    outputs: Vec<&'a mut dyn Output>,
}

impl<'a> Executor<'a> {
    pub fn input(mut self, input: &'a mut dyn Input) -> Self {
        self.input = Some(input);
        self
    }

    pub fn parser(mut self, parser: &'a dyn Parser) -> Self {
        self.parser = Some(parser);
        self
    }

    pub fn generator(
        mut self,
        generator: &'a mut dyn Generator,
        outputs: Vec<&'a mut dyn Output>,
    ) -> Self {
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

        let model = parser.parse(input)?;

        for info in self.generator_infos {
            for output in info.outputs {
                // todo log generating for abc to output xyz
                info.generator.generate(&model, output)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use anyhow::{anyhow, Result};

    use crate::generator::Generator;
    use crate::input::Input;
    use crate::model::{Api, Dto, NamespaceChild, UNDEFINED_NAMESPACE};
    use crate::output::Output;
    use crate::parser::Parser;

    mod execute {
        use anyhow::Result;

        use crate::executor::test::{FakeGenerator, FakeParser};
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
        use crate::executor::test::{FakeGenerator, FakeParser};
        use crate::executor::Executor;
        use crate::{input, output};

        #[test]
        fn missing_input() {
            let result = Executor::default()
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
            let result = Executor::default()
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
            let result = Executor::default()
                .input(&mut input::Buffer::new(parser.test_data(1)))
                .parser(&parser)
                // no generator
                .execute();
            assert!(result.is_err())
        }

        #[test]
        fn missing_output() {
            let parser = FakeParser::default();
            let result = Executor::default()
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
        fn parse<'a>(&self, input: &'a mut dyn Input) -> Result<Api<'a>> {
            Ok(Api {
                name: UNDEFINED_NAMESPACE,
                children: input
                    .next_chunk()
                    .ok_or_else(|| anyhow!("no input data!"))?
                    .split(&self.delimiter)
                    .into_iter()
                    .map(|name| Dto {
                        name,
                        ..Default::default()
                    })
                    .map(NamespaceChild::Dto)
                    .collect::<Vec<NamespaceChild>>(),
            })
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
        fn generate(&mut self, model: &Api, output: &mut dyn Output) -> Result<()> {
            let dto_names = model.dtos().map(|dto| dto.name).collect::<Vec<&str>>();
            output.write_str(&dto_names.join(&self.delimiter))?;
            Ok(())
        }
    }
}
