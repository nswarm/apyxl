use std::borrow::Cow;

use anyhow::{anyhow, Result};
use ariadne::{Color, Label, Report, ReportKind, Source};
use chumsky::prelude::*;
use itertools::Itertools;
use log::debug;

use crate::model::{
    attribute, Api, Attributes, Chunk, Comment, Dto, EnumValueNumber, Field, Namespace,
    NamespaceChild, Rpc, UNDEFINED_NAMESPACE,
};
use crate::parser::Config;
use crate::{model, Input};
use crate::{rust_util, Parser as ApyxlParser};

mod en;
mod ty;

type Error<'a> = extra::Err<Rich<'a, char>>;

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

            let imports = multi_comment()
                .then(use_decl())
                .padded()
                .repeated()
                .collect::<Vec<_>>();

            let children = imports
                .ignore_then(namespace_children(config, namespace(config)).padded())
                .then_ignore(end())
                .parse(data)
                .into_result()
                .map_err(|errs| {
                    let return_err = anyhow!("errors encountered while parsing: {:?}", &errs);
                    report_errors(chunk, data, errs);
                    return_err
                })?;

            builder.merge_from_chunk(
                Api {
                    name: Cow::Borrowed(UNDEFINED_NAMESPACE),
                    children,
                    attributes: Default::default(),
                },
                chunk,
            );
            builder.clear_namespace();
        }

        Ok(())
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Visibility {
    Public,
    Private,
}

impl Visibility {
    fn is_visible(&self, config: &Config) -> bool {
        *self == Visibility::Public || (*self == Visibility::Private && config.enable_parse_private)
    }

    fn filter_child<'a>(
        &self,
        child: NamespaceChild<'a>,
        config: &Config,
    ) -> Option<NamespaceChild<'a>> {
        if self.is_visible(config) {
            Some(child)
        } else {
            None
        }
    }
}

fn use_decl<'a>() -> impl Parser<'a, &'a str, (), Error<'a>> {
    keyword_ex("pub")
        .then(text::whitespace().at_least(1))
        .or_not()
        .then(keyword_ex("use"))
        .then(text::whitespace().at_least(1))
        .then(text::ident().separated_by(just("::")).at_least(1))
        .then(just(';'))
        .ignored()
}

fn field(config: &Config) -> impl Parser<&str, Field, Error> {
    let field = text::ident()
        .then_ignore(just(':').padded())
        .then(ty::ty(config));
    multi_comment()
        .then(attributes().padded())
        .then(field)
        .map(|((comments, user), (name, ty))| Field {
            name,
            ty,
            attributes: Attributes {
                comments,
                user,
                ..Default::default()
            },
        })
}

fn fields(config: &Config) -> impl Parser<&str, Vec<Field>, Error> {
    field(config)
        .separated_by(just(',').padded())
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(just('{').padded(), just('}').padded())
}

fn attributes<'a>() -> impl Parser<'a, &'a str, Vec<attribute::User<'a>>, Error<'a>> {
    let name = text::ident();
    let data = text::ident()
        .then(just('=').padded().ignore_then(text::ident()).or_not())
        .map(|(lhs, rhs)| match rhs {
            None => attribute::UserData::new(None, lhs),
            Some(rhs) => attribute::UserData::new(Some(lhs), rhs),
        });
    let data_list = data
        .separated_by(just(',').padded())
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(just('(').padded(), just(')').padded())
        .or_not();
    name.then(data_list)
        .map(|(name, data)| attribute::User {
            name,
            data: data.unwrap_or(vec![]),
        })
        .separated_by(just(',').padded())
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(just("#[").padded(), just(']').padded())
        .recover_with(skip_then_retry_until(
            none_of(",]").ignored(),
            just(']').ignored(),
        ))
        .or_not()
        .map(|opt| opt.unwrap_or(vec![]))
}

fn visibility<'a>() -> impl Parser<'a, &'a str, Visibility, Error<'a>> {
    keyword_ex("pub")
        .then(text::whitespace().at_least(1))
        .or_not()
        .map(|o| match o {
            None => Visibility::Private,
            Some(_) => Visibility::Public,
        })
}

fn dto(config: &Config) -> impl Parser<&str, (Dto, Visibility), Error> {
    let prefix = keyword_ex("struct").then(text::whitespace().at_least(1));
    let name = text::ident();
    multi_comment()
        .padded()
        .then(attributes().padded())
        .then(visibility())
        .then_ignore(prefix)
        .then(name)
        .then(fields(config))
        .map(|((((comments, user), visibility), name), fields)| {
            (
                Dto {
                    name,
                    fields,
                    attributes: Attributes {
                        comments,
                        user,
                        ..Default::default()
                    },
                },
                visibility,
            )
        })
}

#[derive(Debug, PartialEq, Eq)]
enum ExprBlock<'a> {
    Comment(Comment<'a>),
    Body(&'a str),
    Nested(Vec<ExprBlock<'a>>),
}

/// Parses a block comment starting with `/*` and ending with `*/`. The entire contents will be
/// a single element in the vec. This also does not currently handle indentation very well, so the
/// indentation from the source will be present in the comment data.
///
/// ```
/// /*
/// i am
///     a multiline
/// comment
/// */
/// ```
/// would result in
/// `vec!["i am\n    a multiline\ncomment"]`
fn block_comment<'a>() -> impl Parser<'a, &'a str, Comment<'a>, Error<'a>> {
    any()
        .and_is(just("*/").not())
        .repeated()
        .slice()
        .map(&str::trim)
        .delimited_by(just("/*"), just("*/"))
        .map(|s| {
            if !s.is_empty() {
                Comment::from(vec![s])
            } else {
                Comment::default()
            }
        })
}

/// Parses a line comment where each line starts with `//`. Each line is an element in the returned
/// vec without the prefixed `//`, including all padding and empty lines.
///
/// ```
/// // i am
/// //     a multiline
/// // comment
/// //
/// ```
/// would result in
/// `vec!["i am", "    a multiline", "comment", ""]`
fn line_comment<'a>() -> impl Parser<'a, &'a str, Comment<'a>, Error<'a>> {
    let text = any().and_is(just('\n').not()).repeated().slice();
    let line_start = just("//").then(just(' ').or_not());
    let line = text::inline_whitespace()
        .then(line_start)
        .ignore_then(text)
        .then_ignore(just('\n'));
    line.map(|s| Cow::Borrowed(s))
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .map(|v| v.into())
}

/// Parses a single line or block comment group. Each line is an element in the returned vec.
fn comment<'a>() -> impl Parser<'a, &'a str, Comment<'a>, Error<'a>> {
    choice((line_comment(), block_comment()))
}

/// Parses zero or more [comment]s (which are themselves Vec<&str>) into a Vec.
fn multi_comment<'a>() -> impl Parser<'a, &'a str, Vec<Comment<'a>>, Error<'a>> {
    comment().padded().repeated().collect::<Vec<_>>()
}

fn expr_block<'a>() -> impl Parser<'a, &'a str, Vec<ExprBlock<'a>>, Error<'a>> {
    let body = none_of("{}").repeated().at_least(1).slice().map(&str::trim);
    recursive(|nested| {
        choice((
            comment().boxed().padded().map(ExprBlock::Comment),
            nested.map(ExprBlock::Nested),
            body.map(ExprBlock::Body),
        ))
        .repeated()
        .collect::<Vec<_>>()
        .delimited_by(just('{').padded(), just('}').padded())
        .recover_with(via_parser(nested_delimiters('{', '}', [], |_| vec![])))
    })
}

fn rpc(config: &Config) -> impl Parser<&str, (Rpc, Visibility), Error> {
    let prefix = keyword_ex("fn").then(text::whitespace().at_least(1));
    let name = text::ident();
    let params = field(config)
        .separated_by(just(',').padded().recover_with(skip_then_retry_until(
            any().ignored(),
            one_of(",)").ignored(),
        )))
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(just('(').padded(), just(')').padded());
    let return_type = just("->").ignore_then(ty::ty(config).padded());
    multi_comment()
        .then(attributes().padded())
        .then(visibility())
        .then_ignore(prefix)
        .then(name)
        .then(params)
        .then(return_type.or_not())
        .then_ignore(expr_block().padded())
        .map(
            |(((((comments, user), visibility), name), params), return_type)| {
                (
                    Rpc {
                        name,
                        params,
                        return_type,
                        attributes: Attributes {
                            comments,
                            user,
                            ..Default::default()
                        },
                    },
                    visibility,
                )
            },
        )
}

const INVALID_ENUM_NUMBER: EnumValueNumber = EnumValueNumber::MAX;

fn namespace_children<'a>(
    config: &'a Config,
    namespace: impl Parser<'a, &'a str, (Namespace<'a>, Visibility), Error<'a>>,
) -> impl Parser<'a, &'a str, Vec<NamespaceChild<'a>>, Error<'a>> {
    choice((
        dto(config).map(|(c, v)| (NamespaceChild::Dto(c), v)),
        rpc(config).map(|(c, v)| (NamespaceChild::Rpc(c), v)),
        en::en().map(|(c, v)| (NamespaceChild::Enum(c), v)),
        namespace.map(|(c, v)| (NamespaceChild::Namespace(c), v)),
    ))
    .map(|(child, visibility)| visibility.filter_child(child, config))
    .recover_with(skip_then_retry_until(any().ignored(), just('}').ignored()))
    .repeated()
    .collect::<Vec<_>>()
    .map(|v| v.into_iter().flatten().collect_vec())
}

fn namespace(config: &Config) -> impl Parser<&str, (Namespace, Visibility), Error> {
    recursive(|nested| {
        let prefix = keyword_ex("mod").then(text::whitespace().at_least(1));
        let name = text::ident();
        let body = namespace_children(config, nested)
            .boxed()
            .then_ignore(multi_comment())
            .delimited_by(just('{').padded(), just('}').padded());
        multi_comment()
            .then(attributes().padded())
            .then(visibility())
            .then_ignore(prefix)
            .then(name)
            .then(just(';').padded().map(|_| None).or(body.map(Some)))
            .map(|((((comments, user), visibility), name), children)| {
                (
                    Namespace {
                        name: Cow::Borrowed(name),
                        children: children.unwrap_or(vec![]),
                        attributes: Attributes {
                            comments,
                            user,
                            ..Default::default()
                        },
                    },
                    visibility,
                )
            })
            .boxed()
    })
}

/// Expanded [text::keyword] that has a more informative error.
fn keyword_ex(keyword: &str) -> impl Parser<&str, &str, Error> {
    text::ident()
        .try_map(move |s: &str, span| {
            if s == keyword {
                Ok(())
            } else {
                Err(Rich::custom(span, format!("found unexpected token {}", s)))
            }
        })
        .slice()
}

fn report_errors(chunk: &Chunk, src: &str, errors: Vec<Rich<'_, char>>) {
    let filename = chunk
        .relative_file_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or("unknown".to_string());
    for error in errors {
        Report::build(ReportKind::Error, filename.clone(), error.span().start)
            .with_message(error.to_string())
            .with_label(
                Label::new((filename.clone(), error.span().into_range()))
                    .with_message(error.reason().to_string())
                    .with_color(Color::Red),
            )
            // need "label" feature
            // .with_labels(error.contexts().map(|(label, span)| {
            //     Label::new((filename.clone(), span.into_range()))
            //         .with_message(format!("while parsing this {}", label))
            //         .with_color(Color::Yellow)
            // }))
            .finish()
            .print((filename.clone(), Source::from(src)))
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use anyhow::{anyhow, Result};
    use chumsky::error::Rich;
    use chumsky::Parser;

    use crate::model::{Builder, Comment, UNDEFINED_NAMESPACE};
    use crate::parser::rust::field;
    use crate::parser::Config;
    use crate::test_util::executor::TEST_CONFIG;
    use crate::{input, parser, Parser as ApyxlParser};

    type TestError = Vec<Rich<'static, char>>;
    pub fn wrap_test_err(err: TestError) -> anyhow::Error {
        anyhow!("errors encountered while parsing: {:?}", err)
    }

    #[test]
    fn test_field() -> Result<()> {
        let result = field(&TEST_CONFIG).parse("name: Type");
        let output = result.into_result().map_err(wrap_test_err)?;
        assert_eq!(output.name, "name");
        assert_eq!(
            output.ty.api().unwrap().component_names().last().unwrap(),
            "Type"
        );
        Ok(())
    }

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
        fn private_rpc() {}
        pub enum en {}
        enum private_en {}
        pub struct dto {}
        struct private_dto {}
        pub mod namespace {}
        mod private_namespace {}
        "#,
        );
        let mut builder = Builder::default();
        parser::Rust::default().parse(&TEST_CONFIG, &mut input, &mut builder)?;
        let model = builder.build().unwrap();
        assert_eq!(model.api().name, UNDEFINED_NAMESPACE);
        assert!(model.api().dto("dto").is_some());
        assert!(model.api().rpc("rpc").is_some());
        assert!(model.api().en("en").is_some());
        assert!(model.api().namespace("namespace").is_some());
        assert!(model.api().dto("private_dto").is_some());
        assert!(model.api().rpc("private_rpc").is_some());
        assert!(model.api().en("private_en").is_some());
        assert!(model.api().namespace("private_namespace").is_some());
        // make sure comment after 'use' is attributed to rpc.
        assert_eq!(
            model.api().rpc("rpc").unwrap().attributes.comments,
            vec![Comment::unowned(&["rpc comment"])]
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
        assert!(model.api().namespace("namespace").is_some());
        assert!(model.api().dto("ignored_dto").is_none());
        assert!(model.api().rpc("ignored_rpc").is_none());
        assert!(model.api().en("ignored_en").is_none());
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

    mod namespace {
        use anyhow::Result;
        use chumsky::Parser;

        use crate::model::{attribute, Comment, NamespaceChild};
        use crate::parser::rust::tests::wrap_test_err;
        use crate::parser::rust::{namespace, Visibility};
        use crate::test_util::executor::TEST_CONFIG;

        #[test]
        fn declaration() -> Result<()> {
            let (namespace, _) = namespace(&TEST_CONFIG)
                .parse(
                    r#"
            mod empty;
            "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(namespace.name, "empty");
            assert!(namespace.children.is_empty());
            Ok(())
        }

        #[test]
        fn public() -> Result<()> {
            let (namespace, visibility) = namespace(&TEST_CONFIG)
                .parse(
                    r#"
            pub mod empty;
            "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(namespace.name, "empty");
            assert_eq!(visibility, Visibility::Public);
            Ok(())
        }

        #[test]
        fn private() -> Result<()> {
            let (namespace, visibility) = namespace(&TEST_CONFIG)
                .parse(
                    r#"
            mod empty;
            "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(namespace.name, "empty");
            assert_eq!(visibility, Visibility::Private);
            Ok(())
        }

        #[test]
        fn empty() -> Result<()> {
            let (namespace, _) = namespace(&TEST_CONFIG)
                .parse(
                    r#"
            mod empty {}
            "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(namespace.name, "empty");
            assert!(namespace.children.is_empty());
            Ok(())
        }

        #[test]
        fn with_dto() -> Result<()> {
            let (namespace, _) = namespace(&TEST_CONFIG)
                .parse(
                    r#"
            mod ns {
                pub struct DtoName {}
            }
            "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(namespace.name, "ns");
            assert_eq!(namespace.children.len(), 1);
            match &namespace.children[0] {
                NamespaceChild::Dto(dto) => assert_eq!(dto.name, "DtoName"),
                _ => panic!("wrong child type"),
            }
            Ok(())
        }

        #[test]
        fn nested() -> Result<()> {
            let (namespace, _) = namespace(&TEST_CONFIG)
                .parse(
                    r#"
            mod ns0 {
                mod ns1 {}
            }
            "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(namespace.name, "ns0");
            assert_eq!(namespace.children.len(), 1);
            match &namespace.children[0] {
                NamespaceChild::Namespace(ns) => assert_eq!(ns.name, "ns1"),
                _ => panic!("wrong child type"),
            }
            Ok(())
        }

        #[test]
        fn nested_dto() -> Result<()> {
            let (namespace, _) = namespace(&TEST_CONFIG)
                .parse(
                    r#"
            mod ns0 {
                mod ns1 {
                    pub struct DtoName {}
                }
            }
            "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(namespace.name, "ns0");
            assert_eq!(namespace.children.len(), 1);
            match &namespace.children[0] {
                NamespaceChild::Namespace(ns) => {
                    assert_eq!(ns.name, "ns1");
                    assert_eq!(ns.children.len(), 1);
                    match &ns.children[0] {
                        NamespaceChild::Dto(dto) => assert_eq!(dto.name, "DtoName"),
                        _ => panic!("ns1: wrong child type"),
                    }
                }
                _ => panic!("ns0: wrong child type"),
            }
            Ok(())
        }

        #[test]
        fn comment() -> Result<()> {
            let (namespace, _) = namespace(&TEST_CONFIG)
                .parse(
                    r#"
            // multi
            // line
            // comment
            mod ns {}
            "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(
                namespace.attributes.comments,
                vec![Comment::unowned(&["multi", "line", "comment"])]
            );
            Ok(())
        }

        #[test]
        fn attributes() -> Result<()> {
            let (namespace, _) = namespace(&TEST_CONFIG)
                .parse(
                    r#"
                    #[flag1, flag2]
                    mod ns {}
                    "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(
                namespace.attributes.user,
                vec![
                    attribute::User::new_flag("flag1"),
                    attribute::User::new_flag("flag2"),
                ]
            );
            Ok(())
        }
    }

    mod dto {
        use anyhow::Result;
        use chumsky::Parser;

        use crate::model::{attribute, Comment};
        use crate::parser::rust::tests::wrap_test_err;
        use crate::parser::rust::{dto, Visibility};
        use crate::test_util::executor::TEST_CONFIG;

        #[test]
        fn private() -> Result<()> {
            let (dto, visibility) = dto(&TEST_CONFIG)
                .parse(
                    r#"
            struct StructName {}
            "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(dto.name, "StructName");
            assert_eq!(visibility, Visibility::Private);
            Ok(())
        }

        #[test]
        fn public() -> Result<()> {
            let (dto, visibility) = dto(&TEST_CONFIG)
                .parse(
                    r#"
            pub struct StructName {}
            "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(dto.name, "StructName");
            assert_eq!(dto.fields.len(), 0);
            assert_eq!(visibility, Visibility::Public);
            Ok(())
        }

        #[test]
        fn empty() -> Result<()> {
            let (dto, _) = dto(&TEST_CONFIG)
                .parse(
                    r#"
            struct StructName {}
            "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(dto.name, "StructName");
            assert_eq!(dto.fields.len(), 0);
            Ok(())
        }

        #[test]
        fn multiple_fields() -> Result<()> {
            let (dto, _) = dto(&TEST_CONFIG)
                .parse(
                    r#"
            struct StructName {
                field0: i32,
                field1: f32,
            }
            "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(dto.name, "StructName");
            assert_eq!(dto.fields.len(), 2);
            assert_eq!(dto.fields[0].name, "field0");
            assert_eq!(dto.fields[1].name, "field1");
            Ok(())
        }

        #[test]
        fn comment() -> Result<()> {
            let (dto, _) = dto(&TEST_CONFIG)
                .parse(
                    r#"
            // multi
            // line
            // comment
            struct StructName {}
            "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(
                dto.attributes.comments,
                vec![Comment::unowned(&["multi", "line", "comment"])]
            );
            Ok(())
        }

        #[test]
        fn fields_with_comments() -> Result<()> {
            let (dto, _) = dto(&TEST_CONFIG)
                .parse(
                    r#"
            struct StructName {
                // multi
                // line
                field0: i32, /* comment */ field1: f32,
            }
            "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(dto.name, "StructName");
            assert_eq!(dto.fields.len(), 2);
            assert_eq!(dto.fields[0].name, "field0");
            assert_eq!(dto.fields[1].name, "field1");
            assert_eq!(
                dto.fields[0].attributes.comments,
                vec![Comment::unowned(&["multi", "line"])]
            );
            assert_eq!(
                dto.fields[1].attributes.comments,
                vec![Comment::unowned(&["comment"])]
            );
            Ok(())
        }

        #[test]
        fn attributes() -> Result<()> {
            let (dto, _) = dto(&TEST_CONFIG)
                .parse(
                    r#"
                #[flag1, flag2]
                struct StructName {}
                "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(dto.name, "StructName");
            assert_eq!(
                dto.attributes.user,
                vec![
                    attribute::User::new_flag("flag1"),
                    attribute::User::new_flag("flag2"),
                ]
            );
            Ok(())
        }
    }

    mod rpc {
        use anyhow::Result;
        use chumsky::Parser;

        use crate::model::{attribute, Comment};
        use crate::parser::rust::tests::wrap_test_err;
        use crate::parser::rust::{rpc, Visibility};
        use crate::test_util::executor::TEST_CONFIG;

        #[test]
        fn empty_fn() -> Result<()> {
            let (rpc, _) = rpc(&TEST_CONFIG)
                .parse(
                    r#"
            fn rpc_name() {}
            "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(rpc.name, "rpc_name");
            assert!(rpc.params.is_empty());
            assert!(rpc.return_type.is_none());
            Ok(())
        }

        #[test]
        fn public() -> Result<()> {
            let (rpc, visibility) = rpc(&TEST_CONFIG)
                .parse(
                    r#"
            pub fn rpc_name() {}
            "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(rpc.name, "rpc_name");
            assert!(rpc.params.is_empty());
            assert!(rpc.return_type.is_none());
            assert_eq!(visibility, Visibility::Public);
            Ok(())
        }

        #[test]
        fn private() -> Result<()> {
            let (rpc, visibility) = rpc(&TEST_CONFIG)
                .parse(
                    r#"
            fn rpc_name() {}
            "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(rpc.name, "rpc_name");
            assert_eq!(visibility, Visibility::Private);
            Ok(())
        }

        #[test]
        fn fn_keyword_smushed() {
            let rpc = rpc(&TEST_CONFIG)
                .parse(
                    r#"
            pubfn rpc_name() {}
            "#,
                )
                .into_result();
            assert!(rpc.is_err());
        }

        #[test]
        fn comment() -> Result<()> {
            let (rpc, _) = rpc(&TEST_CONFIG)
                .parse(
                    r#"
            // multi
            // line
            // comment
            fn rpc() {}
            "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(
                rpc.attributes.comments,
                vec![Comment::unowned(&["multi", "line", "comment"])]
            );
            Ok(())
        }

        #[test]
        fn single_param() -> Result<()> {
            let (rpc, _) = rpc(&TEST_CONFIG)
                .parse(
                    r#"
            fn rpc_name(param0: ParamType0) {}
            "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(rpc.params.len(), 1);
            assert_eq!(rpc.params[0].name, "param0");
            assert_eq!(
                rpc.params[0].ty.api().unwrap().component_names().last(),
                Some("ParamType0")
            );
            Ok(())
        }

        #[test]
        fn multiple_params() -> Result<()> {
            let (rpc, _) = rpc(&TEST_CONFIG)
                .parse(
                    r#"
            fn rpc_name(param0: ParamType0, param1: ParamType1, param2: ParamType2) {}
            "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(rpc.params.len(), 3);
            assert_eq!(rpc.params[0].name, "param0");
            assert_eq!(
                rpc.params[0].ty.api().unwrap().component_names().last(),
                Some("ParamType0")
            );
            assert_eq!(rpc.params[1].name, "param1");
            assert_eq!(
                rpc.params[1].ty.api().unwrap().component_names().last(),
                Some("ParamType1")
            );
            assert_eq!(rpc.params[2].name, "param2");
            assert_eq!(
                rpc.params[2].ty.api().unwrap().component_names().last(),
                Some("ParamType2")
            );
            Ok(())
        }

        #[test]
        fn multiple_params_with_comments() -> Result<()> {
            let (rpc, _) = rpc(&TEST_CONFIG)
                .parse(
                    r#"
            fn rpc_name(
                // multi
                // line
                param0: ParamType0, /* comment */ param1: ParamType1,
                // multi
                // line
                // comment
                param2: ParamType2
            ) {}
            "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(rpc.params.len(), 3);
            assert_eq!(
                rpc.params[0].attributes.comments,
                vec![Comment::unowned(&["multi", "line"])]
            );
            assert_eq!(
                rpc.params[1].attributes.comments,
                vec![Comment::unowned(&["comment"])]
            );
            assert_eq!(
                rpc.params[2].attributes.comments,
                vec![Comment::unowned(&["multi", "line", "comment"])]
            );
            Ok(())
        }

        #[test]
        fn multiple_params_weird_spacing_trailing_comma() -> Result<()> {
            let (rpc, _) = rpc(&TEST_CONFIG)
                .parse(
                    r#"
            fn rpc_name(param0: ParamType0      , param1
            :    ParamType1     , param2 :ParamType2
                ,
                ) {}
            "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(rpc.params.len(), 3);
            assert_eq!(rpc.params[0].name, "param0");
            assert_eq!(
                rpc.params[0].ty.api().unwrap().component_names().last(),
                Some("ParamType0")
            );
            assert_eq!(rpc.params[1].name, "param1");
            assert_eq!(
                rpc.params[1].ty.api().unwrap().component_names().last(),
                Some("ParamType1")
            );
            assert_eq!(rpc.params[2].name, "param2");
            assert_eq!(
                rpc.params[2].ty.api().unwrap().component_names().last(),
                Some("ParamType2")
            );
            Ok(())
        }

        #[test]
        fn return_type() -> Result<()> {
            let (rpc, _) = rpc(&TEST_CONFIG)
                .parse(
                    r#"
            fn rpc_name() -> Asdfg {}
            "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(
                rpc.return_type
                    .as_ref()
                    .map(|x| x.api().unwrap().component_names().last()),
                Some(Some("Asdfg"))
            );
            Ok(())
        }

        #[test]
        fn return_type_weird_spacing() -> Result<()> {
            let (rpc, _) = rpc(&TEST_CONFIG)
                .parse(
                    r#"
            fn rpc_name()           ->Asdfg{}
            "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(
                rpc.return_type
                    .as_ref()
                    .map(|x| x.api().unwrap().component_names().last()),
                Some(Some("Asdfg"))
            );
            Ok(())
        }

        #[test]
        fn attributes() -> Result<()> {
            let (rpc, _) = rpc(&TEST_CONFIG)
                .parse(
                    r#"
                #[flag1, flag2]
                fn rpc() {}
                "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(
                rpc.attributes.user,
                vec![
                    attribute::User::new_flag("flag1"),
                    attribute::User::new_flag("flag2"),
                ]
            );
            Ok(())
        }
    }

    mod comments {
        use anyhow::Result;
        use chumsky::Parser;

        use crate::model::Comment;
        use crate::parser::rust::tests::wrap_test_err;
        use crate::parser::rust::{comment, multi_comment, namespace};
        use crate::test_util::executor::TEST_CONFIG;

        #[test]
        fn empty_comment_err() {
            assert!(comment().parse("").into_result().is_err());
        }

        #[test]
        fn line_comment() -> Result<()> {
            let value = comment()
                .parse("// line comment\n")
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(value, Comment::unowned(&["line comment"]));
            Ok(())
        }

        #[test]
        fn line_comment_multi_with_spacing() -> Result<()> {
            let value = comment()
                .parse(
                    r#"//
                // line one
                //     line two
                // line three
                //
"#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(
                value,
                Comment::unowned(&["", "line one", "    line two", "line three", ""])
            );
            Ok(())
        }

        #[test]
        fn test_multi_comment() -> Result<()> {
            let value = multi_comment()
                .parse(
                    r#"
                    /* line one */
                    // line two
                    // line three

                    // line four
                    /* line five */
                    /* line six */
                "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(
                value,
                vec![
                    Comment::unowned(&["line one"]),
                    Comment::unowned(&["line two", "line three"]),
                    Comment::unowned(&["line four"]),
                    Comment::unowned(&["line five"]),
                    Comment::unowned(&["line six"]),
                ]
            );
            Ok(())
        }

        #[test]
        fn block_comment() -> Result<()> {
            let value = comment()
                .parse("/* block comment */")
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(value, Comment::unowned(&["block comment"]));
            Ok(())
        }

        #[test]
        fn line_comments_inside_namespace() -> Result<()> {
            namespace(&TEST_CONFIG)
                .parse(
                    r#"
                    mod ns { // comment
                        // comment

                        // comment
                        // comment
                        // comment
                        struct dto {} // comment
                        // comment
                    }
                    "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            Ok(())
        }

        #[test]
        fn block_comment_inside_namespace() -> Result<()> {
            namespace(&TEST_CONFIG)
                .parse(
                    r#"
                    mod ns { /* comment */
                        /* comment */
                        /* comment */
                        struct dto {} /* comment */
                        /* comment */
                    }
                    "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            Ok(())
        }
    }

    mod expr_block {
        use chumsky::{text, Parser};

        use crate::model::Comment;
        use crate::parser::rust::{expr_block, ExprBlock};

        #[test]
        fn complex() {
            let result = expr_block()
                .parse("{left{inner1_left{inner1}inner1_right}middle{inner2}{inner3}right}")
                .into_result();
            assert_eq!(
                result.unwrap(),
                vec![
                    ExprBlock::Body("left"),
                    ExprBlock::Nested(vec![
                        ExprBlock::Body("inner1_left"),
                        ExprBlock::Nested(vec![ExprBlock::Body("inner1"),]),
                        ExprBlock::Body("inner1_right"),
                    ]),
                    ExprBlock::Body("middle"),
                    ExprBlock::Nested(vec![ExprBlock::Body("inner2"),]),
                    ExprBlock::Nested(vec![ExprBlock::Body("inner3"),]),
                    ExprBlock::Body("right"),
                ]
            );
        }

        #[test]
        fn empty() {
            let result = expr_block().parse("{}").into_result();
            assert_eq!(result.unwrap(), vec![]);
        }

        #[test]
        fn arbitrary_content() {
            let result = expr_block()
                .parse(
                    r#"{
                1234 !@#$%^&*()_+-= asdf
            }"#,
                )
                .into_result();
            assert_eq!(
                result.unwrap(),
                vec![ExprBlock::Body("1234 !@#$%^&*()_+-= asdf")]
            );
        }

        #[test]
        fn line_comment() {
            let result = expr_block()
                .parse(
                    r#"
                    { // don't break! }
                    }"#,
                )
                .into_result();
            assert_eq!(
                result.unwrap(),
                vec![ExprBlock::Comment(Comment::unowned(&["don't break! }"]))],
            );
        }

        #[test]
        fn block_comment() {
            let result = expr_block()
                .parse(
                    r#"{
                    { /* don't break! {{{ */ }
                    }"#,
                )
                .into_result();
            assert_eq!(
                result.unwrap(),
                vec![ExprBlock::Nested(vec![ExprBlock::Comment(
                    Comment::unowned(&["don't break! {{{"])
                )])]
            );
        }

        #[test]
        fn continues_parsing_after() {
            let result = expr_block()
                .padded()
                .ignore_then(text::ident().padded())
                .parse(
                    r#"
                {
                  ignored stuff
                }
                not_ignored
                "#,
                )
                .into_result();
            assert!(result.is_ok(), "parse should not fail");
            assert_eq!(result.unwrap(), "not_ignored");
        }
    }

    mod attributes {
        use chumsky::Parser;

        use crate::model::attribute;
        use crate::model::attribute::UserData;
        use crate::parser::rust::dto;
        use crate::test_util::executor::TEST_CONFIG;

        #[test]
        fn flags() {
            run_test(
                r#"
                    #[flag1, flag2, flag3]
                    struct dto {}
                    "#,
                vec![
                    attribute::User::new_flag("flag1"),
                    attribute::User::new_flag("flag2"),
                    attribute::User::new_flag("flag3"),
                ],
            )
        }

        #[test]
        fn lists() {
            run_test(
                r#"
                    #[attr0(a_one), attr1(a_two, b_two, c_two)]
                    struct dto {}
                    "#,
                vec![
                    attribute::User::new("attr0", vec![UserData::new(None, "a_one")]),
                    attribute::User::new(
                        "attr1",
                        vec![
                            UserData::new(None, "a_two"),
                            UserData::new(None, "b_two"),
                            UserData::new(None, "c_two"),
                        ],
                    ),
                ],
            )
        }

        #[test]
        fn maps() {
            run_test(
                r#"
                    #[attr0(k0 = v0, k1 = v1), attr1(k00 = v00)]
                    struct dto {}
                    "#,
                vec![
                    attribute::User::new(
                        "attr0",
                        vec![
                            UserData::new(Some("k0"), "v0"),
                            UserData::new(Some("k1"), "v1"),
                        ],
                    ),
                    attribute::User::new("attr1", vec![UserData::new(Some("k00"), "v00")]),
                ],
            )
        }

        #[test]
        fn mixed() {
            run_test(
                r#"
                    #[attr0(k0 = v0, k1 = v1), attr1, attr2(one, two, three)]
                    struct dto {}
                    "#,
                vec![
                    attribute::User::new(
                        "attr0",
                        vec![
                            UserData::new(Some("k0"), "v0"),
                            UserData::new(Some("k1"), "v1"),
                        ],
                    ),
                    attribute::User::new_flag("attr1"),
                    attribute::User::new(
                        "attr2",
                        vec![
                            UserData::new(None, "one"),
                            UserData::new(None, "two"),
                            UserData::new(None, "three"),
                        ],
                    ),
                ],
            )
        }

        fn run_test(content: &str, expected: Vec<attribute::User>) {
            let (dto, _) = dto(&TEST_CONFIG).parse(content).into_result().unwrap();
            assert_eq!(dto.attributes.user, expected);
        }
    }
}
