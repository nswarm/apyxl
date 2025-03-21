use std::borrow::Cow;

use anyhow::{anyhow, Result};
use chumsky::prelude::*;
use log::debug;

use crate::model::{Api, UNDEFINED_NAMESPACE};
use crate::parser::error::Error;
use crate::parser::{error, util, Config};
use crate::{model, Input};
use crate::{rust_util, Parser as ApyxlParser};

mod attributes;
mod comment;
mod dto;
mod en;
mod expr_block;
mod namespace;
mod rpc;
mod ty;
mod ty_alias;
mod visibility;

#[derive(Default)]
pub struct Rust {}

impl ApyxlParser for Rust {
    fn parse<'a, I: Input + 'a>(
        &self,
        config: &'a Config,
        input: &'a mut I,
        builder: &mut model::Builder<'a>,
    ) -> Result<()> {
        for (chunk, data) in input.chunks() {
            debug!("parsing chunk {:?}", chunk.relative_file_path);
            if let Some(file_path) = &chunk.relative_file_path {
                for component in rust_util::path_to_entity_id(file_path).component_names() {
                    builder.enter_namespace(component)
                }
            }

            let imports = comment::multi()
                .then(use_decl())
                .padded()
                .repeated()
                .collect::<Vec<_>>();

            let children = imports
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

            builder.merge_from_chunk(
                Api {
                    name: Cow::Borrowed(UNDEFINED_NAMESPACE),
                    children,
                    attributes: Default::default(),
                    is_virtual: false,
                },
                chunk,
            );
            builder.clear_namespace();
        }

        Ok(())
    }
}

fn use_decl<'a>() -> impl Parser<'a, &'a str, (), Error<'a>> {
    let multi_import = text::ident()
        .separated_by(just(',').padded())
        .at_least(1)
        .collect::<Vec<_>>()
        .delimited_by(just('{').padded(), just('}').padded());
    util::keyword_ex("pub")
        .then(text::whitespace().at_least(1))
        .or_not()
        .then(util::keyword_ex("use"))
        .then(text::whitespace().at_least(1))
        .then(
            text::ident()
                .then(just("::"))
                .repeated()
                .collect::<Vec<_>>(),
        )
        .then(choice((
            // ignoring these early so they're all the same out type for the choice.
            multi_import.ignored(),
            text::ident().ignored(),
            just("*").ignored(),
        )))
        .then(just(';'))
        .ignored()
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::model::{Builder, Comment, UNDEFINED_NAMESPACE};
    use crate::parser::Config;
    use crate::test_util::executor::TEST_CONFIG;
    use crate::{input, parser, Parser as ApyxlParser};

    #[test]
    fn root_namespace() -> Result<()> {
        let mut input = input::Buffer::new(
            r#"
        // comment
        use asdf;
        // comment
        // comment
        pub use asdf;
        // rpc comment
        pub fn rpc() {}
        // zzz
        const ignored: &[&str] = &["zz", "xx"];
        // alias comment
        type private_alias = u32;
        // alias comment
        pub type alias = u32;
        fn private_rpc() {}
        pub enum en {}
        enum private_en {}
        pub struct dto {}
        impl dto {
            fn method() {}
        }
        struct private_dto {}
        pub mod namespace {}
        mod private_namespace {}
        pub const asjkdhflakjshdg ignored var;
        // end comment ignored
        "#,
        );
        let mut builder = Builder::default();
        parser::Rust::default().parse(&TEST_CONFIG, &mut input, &mut builder)?;
        let model = builder.build().unwrap();
        assert_eq!(model.api().name, UNDEFINED_NAMESPACE);
        assert!(model.api().dto("dto").is_some(), "dto");
        assert!(model.api().rpc("rpc").is_some(), "rpc");
        assert!(model.api().en("en").is_some(), "en");
        assert!(model.api().ty_alias("alias").is_some(), "alias");
        assert!(model.api().namespace("namespace").is_some(), "namespace");
        assert!(model.api().dto("private_dto").is_some(), "private_dto");
        assert!(model.api().rpc("private_rpc").is_some(), "private_rpc");
        assert!(model.api().en("private_en").is_some(), "private_en");
        assert!(
            model.api().ty_alias("private_alias").is_some(),
            "private_alias"
        );
        assert!(
            model.api().namespace("private_namespace").is_some(),
            "private_namespace"
        );
        assert_eq!(
            model.api().rpc("rpc").unwrap().attributes.comments,
            vec![Comment::unowned(&["rpc comment"])],
            "comment after 'use' attributed to rpc"
        );
        assert!(
            model.api().dto("dto").unwrap().namespace.is_some(),
            "dto impl block ns"
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
            "impl block rpc"
        );
        Ok(())
    }

    #[test]
    fn disabled_parse_private() -> Result<()> {
        let mut input = input::Buffer::new(
            r#"
        // comment
        use asdf;
        // comment
        // comment
        pub use asdf;
        // rpc comment
        pub fn rpc() {}
        fn ignored_rpc() {}
        pub enum en {}
        enum ignored_en {}
        pub struct dto {}
        struct ignored_dto {}
        type ignored_alias = u32;
        pub type alias = u32;
        pub mod namespace {}
        mod ignored_namespace {}
        "#,
        );
        let mut builder = Builder::default();
        let config = Config {
            enable_parse_private: false,
            ..Default::default()
        };
        parser::Rust::default().parse(&config, &mut input, &mut builder)?;
        let model = builder.build().unwrap();
        assert_eq!(model.api().name, UNDEFINED_NAMESPACE);
        assert!(model.api().dto("dto").is_some());
        assert!(model.api().rpc("rpc").is_some());
        assert!(model.api().en("en").is_some());
        assert!(model.api().ty_alias("alias").is_some());
        assert!(model.api().namespace("namespace").is_some());
        assert!(model.api().dto("ignored_dto").is_none());
        assert!(model.api().rpc("ignored_rpc").is_none());
        assert!(model.api().en("ignored_en").is_none());
        assert!(model.api().ty_alias("ignored_alias").is_none());
        assert!(model.api().namespace("ignored_namespace").is_none());
        Ok(())
    }

    mod file_path_to_mod {
        use anyhow::Result;

        use crate::model::{Builder, Chunk, EntityId};
        use crate::test_util::executor::TEST_CONFIG;
        use crate::{input, parser, Parser};

        #[test]
        fn file_path_including_name_without_ext() -> Result<()> {
            let mut input = input::ChunkBuffer::new();
            input.add_chunk(
                Chunk::with_relative_file_path("a/b/c.rs"),
                "pub struct dto {}",
            );
            let mut builder = Builder::default();
            parser::Rust::default().parse(&TEST_CONFIG, &mut input, &mut builder)?;
            let model = builder.build().unwrap();

            let namespace = model
                .api()
                .find_namespace(&EntityId::new_unqualified("a.b.c"));
            assert!(namespace.is_some());
            assert!(namespace.unwrap().dto("dto").is_some());
            Ok(())
        }

        #[test]
        fn ignore_mod_rs() -> Result<()> {
            let mut input = input::ChunkBuffer::new();
            input.add_chunk(
                Chunk::with_relative_file_path("a/b/mod.rs"),
                "pub struct dto {}",
            );
            let mut builder = Builder::default();
            parser::Rust::default().parse(&TEST_CONFIG, &mut input, &mut builder)?;
            let model = builder.build().unwrap();

            let namespace = model
                .api()
                .find_namespace(&EntityId::new_unqualified("a.b"));
            assert!(namespace.is_some());
            assert!(namespace.unwrap().dto("dto").is_some());
            Ok(())
        }

        #[test]
        fn ignore_lib_rs() -> Result<()> {
            let mut input = input::ChunkBuffer::new();
            input.add_chunk(
                Chunk::with_relative_file_path("a/b/lib.rs"),
                "pub struct dto {}",
            );
            let mut builder = Builder::default();
            parser::Rust::default().parse(&TEST_CONFIG, &mut input, &mut builder)?;
            let model = builder.build().unwrap();

            let namespace = model
                .api()
                .find_namespace(&EntityId::new_unqualified("a.b"));
            assert!(namespace.is_some());
            assert!(namespace.unwrap().dto("dto").is_some());
            Ok(())
        }
    }

    mod use_decl {
        use crate::model::Builder;
        use crate::test_util::executor::TEST_CONFIG;
        use crate::{input, parser, Parser};
        use anyhow::Result;

        #[test]
        fn private() -> Result<()> {
            run_test("use asd;")
        }

        #[test]
        fn public() -> Result<()> {
            run_test("pub use asd;")
        }

        #[test]
        fn namespaced() -> Result<()> {
            run_test("use a::b::c::d;")
        }

        #[test]
        fn wildcard() -> Result<()> {
            run_test("use a::b::c::*;")
        }

        #[test]
        fn multi() -> Result<()> {
            run_test("use a::b::c::{asd, efg, xyz};")
        }

        fn run_test(input: &str) -> Result<()> {
            let mut input = input::Buffer::new(input);
            let mut builder = Builder::default();
            parser::Rust::default().parse(&TEST_CONFIG, &mut input, &mut builder)
        }
    }
}
