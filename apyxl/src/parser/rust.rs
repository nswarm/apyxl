use anyhow::Result;
use chumsky::prelude::*;
use chumsky::text::whitespace;

use crate::model::{Api, Dto, Field, Namespace, NamespaceChild, Rpc, TypeRef, UNDEFINED_NAMESPACE};
use crate::Input;
use crate::Parser as ApyxlParser;

type Error<'a> = extra::Err<Simple<'a, char>>;

#[derive(Default)]
pub struct Rust {}

impl ApyxlParser for Rust {
    fn parse<'a>(&self, input: &'a mut dyn Input) -> Result<Api<'a>> {
        // while next_chunk
        // let api = parse(chunk)
        // ApiBuilder.merge(api)

        // let children = children(namespace())
        //     .padded()
        //     .then_ignore(end())
        //     .parse(input.data())
        //     .into_result()
        //     .map_err(|err| anyhow!("errors encountered while parsing: {:?}", err))?;
        Ok(Api {
            name: UNDEFINED_NAMESPACE,
            children: vec![], // children,
        })
    }
}

fn type_ref<'a>() -> impl Parser<'a, &'a str, TypeRef<'a>, Error<'a>> {
    text::ident()
        .separated_by(just("::"))
        .at_least(1)
        .collect::<Vec<_>>()
        .map(|components| TypeRef {
            fully_qualified_type_name: components,
        })
}

fn field<'a>() -> impl Parser<'a, &'a str, Field<'a>, Error<'a>> {
    text::ident()
        .then_ignore(just(':').padded())
        .then(type_ref())
        .padded()
        .map(|(name, ty)| Field { name, ty })
}

fn dto<'a>() -> impl Parser<'a, &'a str, Dto<'a>, Error<'a>> {
    let fields = field()
        .separated_by(just(',').padded())
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(just('{').padded(), just('}').padded());
    let name = text::keyword("struct").padded().ignore_then(text::ident());
    name.then(fields).map(|(name, fields)| Dto { name, fields })
}

fn ignore_fn_body<'a>() -> impl Parser<'a, &'a str, (), Error<'a>> {
    let anything = any().repeated().collect::<Vec<_>>();
    recursive(|nested| nested.delimited_by(just('{'), just('}')).or(anything)).ignored()
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
    let return_type = just("->").ignore_then(whitespace()).ignore_then(type_ref());
    name.then(params)
        .then(return_type.or_not())
        .then_ignore(ignore_fn_body().padded())
        .map(|((name, params), return_type)| Rpc {
            name,
            params,
            return_type,
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
            .map(|(name, children)| Namespace { name, children })
    })
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use chumsky::error::Simple;
    use chumsky::Parser;

    use crate::parser::rust::field;
    use crate::Parser as ApyxlParser;

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
    fn full_parse() -> Result<()> {
        // let input = input::Buffer::new(r#""#);
        // let namespace = Rust::default().parse(&input)?;
        // assert_eq!(namespace.name, UNDEFINED_NAMESPACE);
        // assert!(namespace.children.is_empty());
        todo!()
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
            assert_eq!(rpc.return_type.map(|x| x.name()), Some(Some("Asdfg")));
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
            assert_eq!(rpc.return_type.map(|x| x.name()), Some(Some("Asdfg")));
            Ok(())
        }
    }

    mod fn_body {
        use chumsky::Parser;

        use crate::parser::rust::ignore_fn_body;

        #[test]
        fn empty() {
            let result = ignore_fn_body().parse("{}").into_result();
            assert!(result.is_ok(), "content should be parsed as empty");
        }

        #[test]
        fn arbitrary_content() {
            let result = ignore_fn_body()
                .parse(
                    r#"{
                1234 !@#$%^&*()_+-= asdf
            }"#,
                )
                .into_result();
            assert!(result.is_ok(), "content should be parsed as empty");
        }

        #[test]
        fn brackets() {
            let result = ignore_fn_body()
                .parse(
                    r#"{
                {}
                {{}}
                {{
                }}
                {
                    {
                        {{}
                        {}}
                    }
                }
            }"#,
                )
                .into_result();
            assert!(result.is_ok(), "content should be parsed as empty");
        }

        #[test]
        fn line_comment() {
            let result = ignore_fn_body()
                .parse(
                    r#"
                    { // don't break! {{{
                    }"#,
                )
                .into_result();
            assert!(result.is_ok(), "content should be parsed as empty");
        }

        #[test]
        fn block_comment() {
            let result = ignore_fn_body()
                .parse(
                    r#"{
                    { /* don't break! {{{ */
                    }"#,
                )
                .into_result();
            assert!(result.is_ok(), "content should be parsed as empty");
        }
    }
}
