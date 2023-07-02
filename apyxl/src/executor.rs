use anyhow::{anyhow, Result};
use itertools::Itertools;
use log::{debug, info, log_enabled};
use std::cell::RefCell;
use std::ops::DerefMut;
use std::rc::Rc;

use crate::generator::Generator;
use crate::input::Input;
use crate::model::ValidationError;
use crate::output::Output;
use crate::parser::Parser;
use crate::{model, parser};

type OutputPtr = Rc<RefCell<dyn Output>>;

pub struct Executor<I: Input, P: Parser> {
    input: I,
    parser: P,
    parser_config: Option<parser::Config>,
    generator_infos: Vec<GeneratorInfo>,
}

pub struct GeneratorInfo {
    generator: Box<dyn Generator>,
    outputs: Vec<OutputPtr>,
}

impl<I: Input, P: Parser> Executor<I, P> {
    pub fn new(input: I, parser: P) -> Self {
        Self {
            input,
            parser,
            parser_config: None,
            generator_infos: vec![],
        }
    }

    pub fn parser_config(mut self, config: parser::Config) -> Self {
        self.parser_config = Some(config);
        self
    }

    pub fn generator(mut self, generator: impl Generator + 'static) -> Self {
        self.generator_infos.push(GeneratorInfo {
            generator: Box::new(generator),
            outputs: vec![],
        });
        self
    }

    /// Add an output for the last-added [Generator].
    ///
    /// This method takes complete ownership of the output. If you want access to the output after
    /// execution, use [Executor::output_ptr].
    pub fn output(mut self, output: impl Output + 'static) -> Self {
        self.generator_infos
            .last_mut()
            .expect("no generators added")
            .outputs
            .push(Rc::new(RefCell::new(output)));
        self
    }

    /// Add an output for the last-added [Generator].
    ///
    /// Outputs are `Rc<RefCell<dyn Output>>` which allows you to keep access to the output
    /// for usage after [Executor::execute] is called.
    ///
    /// The output is only borrowed mutably during [Executor::execute].
    pub fn output_ptr(mut self, output: OutputPtr) -> Self {
        self.generator_infos
            .last_mut()
            .expect("no generators added")
            .outputs
            .push(output);
        self
    }

    pub fn execute(mut self) -> Result<()> {
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

        let parser_config = self.parser_config.unwrap_or(Default::default());
        debug!("Parser Config: {:#?}", parser_config);

        info!("Parsing...");
        let mut model_builder = model::Builder::with_config(builder_config());
        self.parser
            .parse(&parser_config, &mut self.input, &mut model_builder)?;

        info!("Validating model...");
        let model = match model_builder.build() {
            Ok(model) => model,
            Err(errors) => {
                return Err(anyhow!(
                    "API validation failed.\n{}",
                    errors_to_string(&errors)
                ))
            }
        };

        for mut info in self.generator_infos {
            for output in info.outputs {
                info!(
                    "Generating for generator '{:?}' to output '{:?}'...",
                    info.generator,
                    output.borrow()
                );
                info.generator
                    .generate(model.view(), output.borrow_mut().deref_mut())?;
            }
        }
        Ok(())
    }
}

fn builder_config() -> model::builder::Config {
    let print = if log_enabled!(log::Level::Trace) {
        model::builder::PreValidatePrint::Debug
    } else if log_enabled!(log::Level::Debug) {
        model::builder::PreValidatePrint::Rust
    } else {
        model::builder::PreValidatePrint::None
    };

    model::builder::Config {
        debug_pre_validate_print: print,
    }
}

fn errors_to_string(errors: &[ValidationError]) -> String {
    errors.iter().map(|e| format!("{}", e)).join("\n")
}

#[cfg(test)]
mod tests {
    use anyhow::{anyhow, Result};
    use std::borrow::Cow;

    use crate::generator::Generator;
    use crate::input::Input;
    use crate::model::{Api, Dto, NamespaceChild, UNDEFINED_NAMESPACE};
    use crate::output::Output;
    use crate::parser::Parser;
    use crate::{model, parser, view};

    mod execute {
        use anyhow::Result;
        use std::cell::RefCell;
        use std::rc::Rc;

        use crate::executor::tests::{FakeGenerator, FakeParser};
        use crate::{input, output, Executor};

        #[test]
        fn happy_path() -> Result<()> {
            let parser = FakeParser::default();
            let input = input::Buffer::new(parser.test_data(1));
            let output = Rc::new(RefCell::new(output::Buffer::default()));
            Executor::new(input, parser.clone())
                .generator(FakeGenerator::default())
                .output_ptr(output.clone())
                .execute()?;
            assert_eq!(output.borrow().to_string(), parser.test_data(1));
            Ok(())
        }

        #[test]
        fn calls_all_generators_with_correct_outputs() -> Result<()> {
            let input_vec = vec![1, 2, 3];
            let parser = FakeParser::new(",");
            let gen0 = FakeGenerator::new("/");
            let gen1 = FakeGenerator::new(":");
            let output0 = Rc::new(RefCell::new(output::Buffer::default()));
            let output1 = Rc::new(RefCell::new(output::Buffer::default()));
            let output2 = Rc::new(RefCell::new(output::Buffer::default()));
            Executor::new(input::Buffer::new(parser.test_data_vec(&input_vec)), parser)
                .generator(gen0.clone())
                .output_ptr(output0.clone())
                .generator(gen1.clone())
                .output_ptr(output1.clone())
                .output_ptr(output2.clone())
                .execute()?;
            assert_eq!(output0.borrow().to_string(), gen0.expected(&input_vec));
            assert_eq!(output1.borrow().to_string(), gen1.expected(&input_vec));
            assert_eq!(output2.borrow().to_string(), gen1.expected(&input_vec));
            Ok(())
        }
    }

    mod validation {
        use crate::executor::tests::{FakeGenerator, FakeParser};
        use crate::executor::Executor;
        use crate::input;

        #[test]
        fn missing_generator() {
            let parser = FakeParser::default();
            let result = Executor::new(input::Buffer::new(parser.test_data(1)), parser)
                // no generator
                .execute();
            assert!(result.is_err())
        }

        #[test]
        fn missing_output() {
            let parser = FakeParser::default();
            let result = Executor::new(input::Buffer::new(parser.test_data(1)), parser)
                .generator(FakeGenerator::default())
                // no output
                .execute();
            assert!(result.is_err())
        }
    }

    #[derive(Default, Clone)]
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
        fn parse<'a, I: Input + 'a>(
            &self,
            _: &'a parser::Config,
            input: &'a mut I,
            builder: &mut model::Builder<'a>,
        ) -> Result<()> {
            builder.merge(Api {
                name: Cow::Borrowed(UNDEFINED_NAMESPACE),
                children: input
                    .chunks()
                    .get(0)
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
            Ok(())
        }
    }

    #[derive(Debug, Default, Clone)]
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
        fn generate(&mut self, model: view::Model, output: &mut dyn Output) -> Result<()> {
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
