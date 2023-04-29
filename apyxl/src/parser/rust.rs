use anyhow::{anyhow, Result};
use chumsky::prelude::*;
use chumsky::text::whitespace;
use log::debug;
use std::borrow::Cow;
use std::path::Path;

use crate::model::{
    Api, Dto, EntityId, Field, Namespace, NamespaceChild, Rpc, UNDEFINED_NAMESPACE,
};
use crate::Parser as ApyxlParser;
use crate::{model, Input};

type Error<'a> = extra::Err<Simple<'a, char>>;

#[derive(Default)]
pub struct Rust {}

impl ApyxlParser for Rust {
    fn parse<'a, I: Input + 'a>(&self, input: &'a mut I) -> Result<model::Builder<'a>> {
        let mut builder = model::Builder::default();

        while let Some((chunk, data)) = input.next_chunk() {
            debug!("parsing chunk {:?}", chunk.relative_file_path);
            if let Some(file_path) = &chunk.relative_file_path {
                for component in path_elders_iter(file_path) {
                    builder.enter_namespace(&component)
                }
            }

            let children = namespace_children(namespace())
                .padded()
                .then_ignore(end())
                .parse(&data)
                .into_result()
                .map_err(|err| anyhow!("errors encountered while parsing: {:?}", err))?;

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

        Ok(builder)
    }
}

/// Iterate over path except for self from front to back.
fn path_elders_iter<'a>(path: &'a Path) -> impl Iterator<Item = Cow<'a, str>> + 'a {
    path.iter()
        .filter(move |p| p != &path.file_name().unwrap())
        .map(|p| p.to_string_lossy())
}

fn entity_id<'a>() -> impl Parser<'a, &'a str, EntityId, Error<'a>> {
    text::ident()
        .separated_by(just("::"))
        .at_least(1)
        .collect::<Vec<_>>()
        .map(|components| EntityId {
            path: components
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<String>>(),
        })
}

fn field<'a>() -> impl Parser<'a, &'a str, Field<'a>, Error<'a>> {
    text::ident()
        .then_ignore(just(':').padded())
        .then(entity_id())
        .padded()
        .map(|(name, ty)| Field {
            name,
            ty,
            attributes: Default::default(),
        })
}

fn dto<'a>() -> impl Parser<'a, &'a str, Dto<'a>, Error<'a>> {
    let fields = field()
        .separated_by(just(',').padded())
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(just('{').padded(), just('}').padded());
    let name = text::keyword("struct").padded().ignore_then(text::ident());
    name.then(fields).map(|(name, fields)| Dto {
        name,
        fields,
        attributes: Default::default(),
    })
}

#[derive(Debug, PartialEq, Eq)]
enum ExprBlock<'a> {
    Comment(&'a str),
    Body(&'a str),
    Nested(Vec<ExprBlock<'a>>),
}

fn expr_block<'a>() -> impl Parser<'a, &'a str, Vec<ExprBlock<'a>>, Error<'a>> {
    let block_comment = any()
        .and_is(just("*/").not())
        .repeated()
        .slice()
        .map(&str::trim)
        .delimited_by(just("/*"), just("*/"));
    let line_comment = just("//").ignore_then(none_of('\n').repeated().slice().map(&str::trim));
    let body = none_of("{}").repeated().at_least(1).slice().map(&str::trim);
    recursive(|nested| {
        choice((
            block_comment.padded().map(ExprBlock::Comment),
            line_comment.padded().map(ExprBlock::Comment),
            nested.map(ExprBlock::Nested),
            body.map(ExprBlock::Body),
        ))
        .repeated()
        .collect::<Vec<_>>()
        .delimited_by(just('{').padded(), just('}').padded())
    })
}

fn rpc<'a>() -> impl Parser<'a, &'a str, Rpc<'a>, Error<'a>> {
    let fn_keyword = text::keyword("pub")
        .then(whitespace().at_least(1))
        .or_not()
        .then(text::keyword("fn"));
    let name = fn_keyword.padded().ignore_then(text::ident());
    let params = field()
        .separated_by(just(',').padded())
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(just('(').padded(), just(')').padded());
    let return_type = just("->")
        .ignore_then(whitespace())
        .ignore_then(entity_id());
    name.then(params)
        .then(return_type.or_not())
        .then_ignore(expr_block().padded())
        .map(|((name, params), return_type)| Rpc {
            name,
            params,
            return_type,
            attributes: Default::default(),
        })
}

fn namespace_children<'a>(
    namespace: impl Parser<'a, &'a str, Namespace<'a>, Error<'a>>,
) -> impl Parser<'a, &'a str, Vec<NamespaceChild<'a>>, Error<'a>> {
    choice((
        dto().padded().map(NamespaceChild::Dto),
        rpc().padded().map(NamespaceChild::Rpc),
        namespace.padded().map(NamespaceChild::Namespace),
    ))
    .repeated()
    .collect::<Vec<_>>()
}

fn namespace<'a>() -> impl Parser<'a, &'a str, Namespace<'a>, Error<'a>> {
    recursive(|nested| {
        let mod_keyword = text::keyword("pub")
            .then(whitespace().at_least(1))
            .or_not()
            .then(text::keyword("mod"));
        let body = namespace_children(nested)
            .boxed()
            .delimited_by(just('{').padded(), just('}').padded());
        mod_keyword
            .padded()
            .ignore_then(text::ident())
            .then(body)
            .map(|(name, children)| Namespace {
                name: Cow::Borrowed(name),
                children,
                attributes: Default::default(),
            })
    })
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use chumsky::error::Simple;
    use chumsky::Parser;

    use crate::model::UNDEFINED_NAMESPACE;
    use crate::parser::rust::field;
    use crate::{input, parser, Parser as ApyxlParser};

    type TestError = Vec<Simple<'static, char>>;

    #[test]
    fn test_field() -> Result<(), TestError> {
        let result = field().parse("name: Type");
        let output = result.into_result()?;
        assert_eq!(output.name, "name");
        assert_eq!(output.ty.name().unwrap(), "Type");
        Ok(())
    }

    #[test]
    fn root_namespace() -> Result<()> {
        let mut input = input::Buffer::new(
            r#"
        fn rpc() {}
        struct dto {}
        mod namespace {}
        "#,
        );
        let model = parser::Rust::default().parse(&mut input)?.build().unwrap();
        assert_eq!(model.api().name, UNDEFINED_NAMESPACE);
        assert!(model.api().dto("dto").is_some());
        assert!(model.api().rpc("rpc").is_some());
        assert!(model.api().namespace("namespace").is_some());
        Ok(())
    }

    mod namespace {
        use chumsky::Parser;

        use crate::model::NamespaceChild;
        use crate::parser::rust::namespace;
        use crate::parser::rust::tests::TestError;

        #[test]
        fn empty() -> Result<(), TestError> {
            let namespace = namespace()
                .parse(
                    r#"
            mod empty {}
            "#,
                )
                .into_result()?;
            assert_eq!(namespace.name, "empty");
            assert!(namespace.children.is_empty());
            Ok(())
        }

        #[test]
        fn with_dto() -> Result<(), TestError> {
            let namespace = namespace()
                .parse(
                    r#"
            mod ns {
                struct DtoName {}
            }
            "#,
                )
                .into_result()?;
            assert_eq!(namespace.name, "ns");
            assert_eq!(namespace.children.len(), 1);
            match &namespace.children[0] {
                NamespaceChild::Dto(dto) => assert_eq!(dto.name, "DtoName"),
                _ => panic!("wrong child type"),
            }
            Ok(())
        }

        #[test]
        fn nested() -> Result<(), TestError> {
            let namespace = namespace()
                .parse(
                    r#"
            mod ns0 {
                mod ns1 {}
            }
            "#,
                )
                .into_result()?;
            assert_eq!(namespace.name, "ns0");
            assert_eq!(namespace.children.len(), 1);
            match &namespace.children[0] {
                NamespaceChild::Namespace(ns) => assert_eq!(ns.name, "ns1"),
                _ => panic!("wrong child type"),
            }
            Ok(())
        }

        #[test]
        fn nested_dto() -> Result<(), TestError> {
            let namespace = namespace()
                .parse(
                    r#"
            mod ns0 {
                mod ns1 {
                    struct DtoName {}
                }
            }
            "#,
                )
                .into_result()?;
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
    }

    mod dto {
        use chumsky::Parser;

        use crate::parser::rust::dto;
        use crate::parser::rust::tests::TestError;

        #[test]
        fn empty() -> Result<(), TestError> {
            let dto = dto()
                .parse(
                    r#"
            struct StructName {}
            "#,
                )
                .into_result()?;
            assert_eq!(dto.name, "StructName");
            assert_eq!(dto.fields.len(), 0);
            Ok(())
        }

        #[test]
        fn multiple_fields() -> Result<(), TestError> {
            let dto = dto()
                .parse(
                    r#"
            struct StructName {
                field0: i32,
                field1: f32,
            }
            "#,
                )
                .into_result()?;
            assert_eq!(dto.name, "StructName");
            assert_eq!(dto.fields.len(), 2);
            assert_eq!(dto.fields[0].name, "field0");
            assert_eq!(dto.fields[1].name, "field1");
            Ok(())
        }
    }

    mod rpc {
        use chumsky::Parser;

        use crate::parser::rust::rpc;
        use crate::parser::rust::tests::TestError;

        #[test]
        fn empty_fn() -> Result<(), TestError> {
            let rpc = rpc()
                .parse(
                    r#"
            fn rpc_name() {}
            "#,
                )
                .into_result()?;
            assert_eq!(rpc.name, "rpc_name");
            assert!(rpc.params.is_empty());
            assert!(rpc.return_type.is_none());
            Ok(())
        }

        #[test]
        fn pub_fn() -> Result<(), TestError> {
            let rpc = rpc()
                .parse(
                    r#"
            pub fn rpc_name() {}
            "#,
                )
                .into_result()?;
            assert_eq!(rpc.name, "rpc_name");
            assert!(rpc.params.is_empty());
            assert!(rpc.return_type.is_none());
            Ok(())
        }

        #[test]
        fn fn_keyword_smushed() {
            let rpc = rpc()
                .parse(
                    r#"
            pubfn rpc_name() {}
            "#,
                )
                .into_result();
            assert!(rpc.is_err());
        }

        #[test]
        fn single_param() -> Result<(), TestError> {
            let rpc = rpc()
                .parse(
                    r#"
            fn rpc_name(param0: ParamType0) {}
            "#,
                )
                .into_result()?;
            assert_eq!(rpc.params.len(), 1);
            assert_eq!(rpc.params[0].name, "param0");
            assert_eq!(rpc.params[0].ty.name(), Some("ParamType0"));
            Ok(())
        }

        #[test]
        fn multiple_params() -> Result<(), TestError> {
            let rpc = rpc()
                .parse(
                    r#"
            fn rpc_name(param0: ParamType0, param1: ParamType1, param2: ParamType2) {}
            "#,
                )
                .into_result()?;
            assert_eq!(rpc.params.len(), 3);
            assert_eq!(rpc.params[0].name, "param0");
            assert_eq!(rpc.params[0].ty.name(), Some("ParamType0"));
            assert_eq!(rpc.params[1].name, "param1");
            assert_eq!(rpc.params[1].ty.name(), Some("ParamType1"));
            assert_eq!(rpc.params[2].name, "param2");
            assert_eq!(rpc.params[2].ty.name(), Some("ParamType2"));
            Ok(())
        }

        #[test]
        fn multiple_params_weird_spacing_trailing_comma() -> Result<(), TestError> {
            let rpc = rpc()
                .parse(
                    r#"
            fn rpc_name(param0: ParamType0      , param1
            :    ParamType1     , param2 :ParamType2
                ,
                ) {}
            "#,
                )
                .into_result()?;
            assert_eq!(rpc.params.len(), 3);
            assert_eq!(rpc.params[0].name, "param0");
            assert_eq!(rpc.params[0].ty.name(), Some("ParamType0"));
            assert_eq!(rpc.params[1].name, "param1");
            assert_eq!(rpc.params[1].ty.name(), Some("ParamType1"));
            assert_eq!(rpc.params[2].name, "param2");
            assert_eq!(rpc.params[2].ty.name(), Some("ParamType2"));
            Ok(())
        }

        #[test]
        fn return_type() -> Result<(), TestError> {
            let rpc = rpc()
                .parse(
                    r#"
            fn rpc_name() -> Asdfg {}
            "#,
                )
                .into_result()?;
            assert_eq!(
                rpc.return_type.as_ref().map(|x| x.name()),
                Some(Some("Asdfg"))
            );
            Ok(())
        }

        #[test]
        fn return_type_weird_spacing() -> Result<(), TestError> {
            let rpc = rpc()
                .parse(
                    r#"
            fn rpc_name()           ->Asdfg{}
            "#,
                )
                .into_result()?;
            assert_eq!(
                rpc.return_type.as_ref().map(|x| x.name()),
                Some(Some("Asdfg"))
            );
            Ok(())
        }
    }

    mod expr_block {
        use chumsky::{text, Parser};

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
            assert_eq!(result.unwrap(), vec![ExprBlock::Comment("don't break! }")]);
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
                    "don't break! {{{"
                )]),]
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
}
