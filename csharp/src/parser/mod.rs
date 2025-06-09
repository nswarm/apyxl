use std::borrow::Cow;

use anyhow::{Result, anyhow};
use chumsky::prelude::*;
use log::debug;

use apyxl::model::{Api, UNDEFINED_NAMESPACE};
use apyxl::parser::error::Error;
use apyxl::parser::{Config, error, util};
use apyxl::{Input, model};

mod attributes;
mod comment;
mod dto;
mod en;
mod expr_block;
mod field;
mod is_static;
mod namespace;
mod rpc;
mod ty;
mod ty_alias;
mod visibility;

#[derive(Default)]
pub struct CSharpParser {}

impl apyxl::Parser for CSharpParser {
    fn parse<'a, I: Input + 'a>(
        &self,
        config: &'a Config,
        input: &'a mut I,
        builder: &mut model::Builder<'a>,
    ) -> Result<()> {
        for (chunk, data) in input.chunks() {
            debug!("parsing chunk {:?}", chunk.relative_file_path);

            let dealiased = remove_aliases(data);

            let imports = using().padded().repeated().collect::<Vec<_>>();

            let children = imports
                .ignore_then(assembly_definitions())
                .ignore_then(
                    namespace::children(config, namespace::parser(config), end().ignored())
                        .padded(),
                )
                .then_ignore(end())
                .parse(data)
                .into_result()
                .map_err(|errs| {
                    let return_err = anyhow!("errors encountered while parsing: {:?}", &errs);
                    error::report_errors(chunk, data, errs.clone());
                    return_err
                })?;

            let api = Api {
                name: Cow::Borrowed(UNDEFINED_NAMESPACE),
                children,
                attributes: Default::default(),
                is_virtual: false,
            };
            builder.merge_from_chunk(api, chunk);
        }

        Ok(())
    }
}

/// Parses any `using X = Y;` statements, then uses those as find+replace input to modify the
/// input data to remove all type aliases, returning the modified data.
///
/// This is necessary because C# doesn't have 'real' typedefs, it only has file-local using
/// statements. This maybe isn't the most performant way to do this, but it is pretty simple.
fn remove_aliases(data: &str) -> Cow<str> {
    // Using statements must be at the beginning of the file.

    /// hmmmmmmmmmm.... this isn't actually that great lol...
    /// because what if I have like...
    /// using Field = int;
    /// class Dto {
    ///     Field Field;
    /// }
    ///
    /// that will result in
    /// class Dto {
    ///     int int;
    /// }
    ///
    /// which is definitely an error.... fffff
    // todo
    // ok... I need a better way of walking the entire hierarchy for types that guarantees
    // that when I add new type locations, those are hit too by everything....
    // maybe like namespace.types() -> Iterator<Item = TypeRef> or smth
    // ........ NOW WE'RE TLAKING
    Cow::Owned(data.to_string())
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct LocalAlias<'a> {
    find: &'a str,
    replace: &'a str,
}

fn using<'a>() -> impl Parser<'a, &'a str, Option<LocalAlias<'a>>, Error<'a>> {
    let find = text::ident().separated_by(just(".")).slice();
    let replace = any()
        .and_is(just(";").not())
        .repeated()
        .slice()
        .map(&str::trim);
    comment::multi()
        .ignore_then(util::keyword_ex("using"))
        .ignore_then(text::whitespace().at_least(1))
        .ignore_then(find)
        .then(just("=").padded().ignore_then(replace).or_not())
        .then_ignore(just(';').padded())
        .map(|(find, replace)| replace.map(|replace| LocalAlias { find, replace }))
}

fn assembly_definitions<'a>() -> impl Parser<'a, &'a str, (), Error<'a>> {
    let asmdef = util::keyword_ex("assembly")
        .then(just(":").padded())
        .then(any().and_is(just("]").not()).repeated().slice())
        .delimited_by(just("[").padded(), just("]").padded());
    comment::multi().then(asmdef).repeated().ignored()
}

#[cfg(test)]
mod tests {
    use crate::parser::{CSharpParser, assembly_definitions};
    use anyhow::Result;
    use apyxl::model::{Builder, UNDEFINED_NAMESPACE};
    use apyxl::parser::Config;
    use apyxl::test_util::executor::TEST_CONFIG;
    use apyxl::{Parser, input};
    use chumsky::Parser as ChumskyParser;
    use itertools::Itertools;

    #[test]
    fn root_namespace() -> Result<()> {
        let mut input = input::Buffer::new(
            r#"
        // comment
        using asdf;
        // comment
        // comment
        using asdf.x.y.z;
        // alias comment
        using private_alias = u32;
        public class dto {
            public void method() {}
        }
        private struct private_dto {}
        namespace SomeNamespace {}
        namespace Some.Other.Namespace {}
        enum private_en {}
        public enum en {}
        /// rpc comment
        public void rpc() {}
        private void private_rpc() {}
        // end comment ignored
        "#,
        );
        let mut builder = Builder::default();
        CSharpParser::default().parse(&TEST_CONFIG, &mut input, &mut builder)?;
        let model = builder.build().unwrap();
        assert_eq!(model.api().name, UNDEFINED_NAMESPACE);
        assert!(model.api().dto("dto").is_some(), "dto");
        assert!(model.api().rpc("rpc").is_some(), "rpc");
        assert!(model.api().en("en").is_some(), "en");
        assert!(
            model.api().namespace("SomeNamespace").is_some(),
            "SomeNamespace"
        );
        assert!(model.api().dto("private_dto").is_some(), "private_dto");
        assert!(model.api().rpc("private_rpc").is_some(), "private_rpc");
        assert!(model.api().en("private_en").is_some(), "private_en");
        assert!(
            model.api().dto("dto").unwrap().namespace.is_some(),
            "dto scope namespace"
        );
        assert!(
            model
                .api()
                .dto("dto")
                .unwrap()
                .namespace
                .as_ref()
                .unwrap()
                .rpc("method")
                .is_some(),
            "dto scope rpc"
        );
        Ok(())
    }

    #[test]
    fn disabled_parse_private() -> Result<()> {
        let mut input = input::Buffer::new(
            r#"
        public enum en {}
        enum ignored_en {}
        public struct dto {
            // rpc comment
            public void rpc() {}
            private void ignored_rpc() {}
        }
        struct ignored_dto {
            public void ignored_rpc() {}
        }
        "#,
        );
        let mut builder = Builder::default();
        let config = Config {
            enable_parse_private: false,
            ..Default::default()
        };
        CSharpParser::default().parse(&config, &mut input, &mut builder)?;
        let model = builder.build().unwrap();
        assert_eq!(model.api().name, UNDEFINED_NAMESPACE);
        let dto = model.api().dto("dto");
        assert!(dto.is_some());
        let dto_ns = dto.unwrap().namespace.as_ref().expect("dto namespace");
        assert!(dto_ns.rpc("rpc").is_some());
        assert!(dto_ns.rpc("ignored_rpc").is_none());
        assert!(model.api().en("en").is_some());
        assert!(model.api().dto("ignored_dto").is_none());
        assert!(model.api().en("ignored_en").is_none());
        Ok(())
    }

    #[test]
    fn assembly_definition_parser() {
        let result = assembly_definitions()
            .parse(
                r#"
            // comment
            [assembly: AssemblyVersion(123.123.123.123)]
            "#,
            )
            .into_result();
        if result.is_err() {
            println!("{}", result.unwrap_err().iter().join(","));
            panic!("error parsing");
        }
    }

    #[test]
    fn ignore_assembly_definitions() -> Result<()> {
        let mut input = input::Buffer::new(
            r#"
        [assembly: blah blahsd () <>.sd.as=++]
        // comment
        [assembly: AssemblyVersion(123.123.123.123)]
        // comment
        // comment
        [assembly: "zzz"]
        "#,
        );
        let mut builder = Builder::default();
        CSharpParser::default().parse(&TEST_CONFIG, &mut input, &mut builder)
    }

    mod using {
        use crate::parser::CSharpParser;
        use anyhow::Result;
        use apyxl::model::{Builder, Type};
        use apyxl::test_util::executor::TEST_CONFIG;
        use apyxl::{Parser, input, parser};

        #[test]
        fn import() -> Result<()> {
            assert_no_parse_errors("using asd;")
        }

        #[test]
        fn namespaced() -> Result<()> {
            assert_no_parse_errors("using a.b.c.d;")
        }

        #[test]
        fn alias_replaces_types() -> Result<()> {
            let input = r#"
            using BlahType = string;
            class Dto {
                BlahType bt;
                BlahType func(BlahType bt) {}
            }
            "#;
            let mut input = input::Buffer::new(input);
            let mut builder = Builder::default();
            CSharpParser::default().parse(&TEST_CONFIG, &mut input, &mut builder)?;
            let model = builder.build().unwrap();
            let api = model.api();

            let dto = api.dto("Dto").unwrap();
            let field = dto.field("bt").unwrap();
            let rpc = dto.rpc("func").unwrap();
            assert_eq!(field.ty.value, Type::String);
            assert_eq!(rpc.return_type.as_ref().unwrap().value, Type::String);
            Ok(())
        }

        fn assert_no_parse_errors(input: &str) -> Result<()> {
            let mut input = input::Buffer::new(input);
            let mut builder = Builder::default();
            CSharpParser::default().parse(&TEST_CONFIG, &mut input, &mut builder)
        }
    }
    mod using_alias {
        use crate::parser::{LocalAlias, using};
        use anyhow::Result;
        use apyxl::parser::test_util::wrap_test_err;
        use chumsky::Parser as ChumskyParser;

        #[test]
        fn non_alias_returns_none() -> Result<()> {
            assert_local_alias(r#"using a.b.c;"#, None)
        }

        #[test]
        fn alias() -> Result<()> {
            assert_local_alias(
                r#"using a = b;"#,
                Some(LocalAlias {
                    find: "a",
                    replace: "b",
                }),
            )
        }

        #[test]
        fn trims_alias() -> Result<()> {
            assert_local_alias(
                r#"using a =   b      ;"#,
                Some(LocalAlias {
                    find: "a",
                    replace: "b",
                }),
            )
        }

        #[test]
        fn complex_alias() -> Result<()> {
            assert_local_alias(
                r#"using a = b. dsa sd()^*#@ vyzu;"#,
                Some(LocalAlias {
                    find: "a",
                    replace: "b. dsa sd()^*#@ vyzu",
                }),
            )
        }

        fn assert_local_alias(input: &str, expected: Option<LocalAlias>) -> Result<()> {
            let actual = using().parse(input).into_result().map_err(wrap_test_err)?;
            assert_eq!(actual, expected);
            Ok(())
        }
    }
}
