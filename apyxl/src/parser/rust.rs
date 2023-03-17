use anyhow::{anyhow, Result};
use chumsky::prelude::*;
use chumsky::text::whitespace;

use crate::model::{Api, Dto, DtoRef, Field, Rpc, Segment};
use crate::Input;
use crate::Parser as ApyxlParser;

type Error<'a> = extra::Err<Simple<'a, char>>;

#[derive(Default)]
pub struct Rust {}

impl ApyxlParser for Rust {
    fn parse<'a>(&self, input: &'a dyn Input) -> Result<Api<'a>> {
        api()
            .parse(input.data())
            .into_result()
            .map_err(|err| anyhow!("errors encountered while parsing: {:?}", err))
    }
}

fn dto_ref<'a>() -> impl Parser<'a, &'a str, DtoRef<'a>, Error<'a>> {
    // todo type can't be ident (e.g. generics vec/map)
    // todo package pathing
    // todo reference one or more other types (and be able to cross ref that in api)
    text::ident().map(|x: &str| DtoRef { name: x })
}

fn field<'a>() -> impl Parser<'a, &'a str, Field<'a>, Error<'a>> {
    text::ident()
        .then_ignore(just(':').padded())
        .then(dto_ref())
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
    let return_type = just("->").ignore_then(whitespace()).ignore_then(dto_ref());
    name.then(params)
        .then(return_type.or_not())
        .then_ignore(ignore_fn_body().padded())
        .map(|((name, params), return_type)| Rpc {
            name,
            params,
            return_type,
        })
}

fn api<'a>() -> impl Parser<'a, &'a str, Api<'a>, Error<'a>> {
    let segments = choice((
        dto().padded().map(Segment::Dto),
        rpc().padded().map(Segment::Rpc),
    ))
    .repeated()
    .collect::<Vec<_>>();
    segments
        .padded()
        .then(end())
        .map(|(segments, _)| Api { segments })
}

#[cfg(test)]
mod test {
    use crate::parser::rust::{dto, field};
    use chumsky::error::Simple;
    use chumsky::Parser;

    type TestError = Vec<Simple<'static, char>>;

    #[test]
    fn test_field() -> Result<(), TestError> {
        let result = field().parse("name: Type");
        let output = result.into_result()?;
        assert_eq!(output.name, "name");
        assert_eq!(output.ty.name, "Type");
        Ok(())
    }

    #[test]
    fn test_dto() -> Result<(), TestError> {
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

    mod rpc {
        use crate::parser::rust::rpc;
        use crate::parser::rust::test::TestError;
        use chumsky::Parser;

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
        fn ignore_fn_body() -> Result<(), TestError> {
            let rpc = rpc()
                .parse(
                    r#"
            fn rpc_name() {


                        1234 !@#$%^&*()_+-= asdf

             }
            "#,
                )
                .into_result()?;
            assert_eq!(rpc.name, "rpc_name");
            assert!(rpc.params.is_empty());
            assert!(rpc.return_type.is_none());
            Ok(())
        }

        #[test]
        fn ignore_brackets_in_fn_body() -> Result<(), TestError> {
            let rpc = rpc()
                .parse(
                    r#"
            fn rpc_name() {
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
            }
            "#,
                )
                .into_result()?;
            assert_eq!(rpc.name, "rpc_name");
            assert!(rpc.params.is_empty());
            assert!(rpc.return_type.is_none());
            Ok(())
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
            assert_eq!(rpc.params[0].ty.name, "ParamType0");
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
            assert_eq!(rpc.params[0].ty.name, "ParamType0");
            assert_eq!(rpc.params[1].name, "param1");
            assert_eq!(rpc.params[1].ty.name, "ParamType1");
            assert_eq!(rpc.params[2].name, "param2");
            assert_eq!(rpc.params[2].ty.name, "ParamType2");
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
            assert_eq!(rpc.params[0].ty.name, "ParamType0");
            assert_eq!(rpc.params[1].name, "param1");
            assert_eq!(rpc.params[1].ty.name, "ParamType1");
            assert_eq!(rpc.params[2].name, "param2");
            assert_eq!(rpc.params[2].ty.name, "ParamType2");
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
            assert_eq!(rpc.return_type.map(|x| x.name), Some("Asdfg"));
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
            assert_eq!(rpc.return_type.map(|x| x.name), Some("Asdfg"));
            Ok(())
        }
    }
}
