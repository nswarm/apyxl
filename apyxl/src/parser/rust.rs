use std::borrow::Cow;

use anyhow::{anyhow, Result};
use chumsky::error;
use chumsky::prelude::*;
use chumsky::text::whitespace;
use log::debug;

use crate::model::{
    Api, Dto, EntityId, Enum, EnumValue, EnumValueNumber, Field, Namespace, NamespaceChild, Rpc,
    Type, UNDEFINED_NAMESPACE,
};
use crate::parser::Config;
use crate::{model, Input};
use crate::{rust_util, Parser as ApyxlParser};

type Error<'a> = extra::Err<Simple<'a, char>>;

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
                for component in rust_util::path_to_entity_id(file_path).path {
                    builder.enter_namespace(&component)
                }
            }

            let children = choice((use_decl().ignored(), comment().ignored()))
                .padded()
                .repeated()
                .collect::<Vec<_>>()
                .ignore_then(namespace_children(&config, namespace(&config)).padded())
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

        Ok(())
    }
}

const ALLOWED_TYPE_NAME_CHARS: &str = "_&<>";

fn type_name<'a>() -> impl Parser<'a, &'a str, &'a str, Error<'a>> {
    any()
        // first char
        .filter(|c: &char| c.is_ascii_alphabetic() || ALLOWED_TYPE_NAME_CHARS.contains(*c))
        // remaining chars
        .then(
            any()
                .filter(|c: &char| c.is_ascii_alphanumeric() || *c == '_')
                .repeated(),
        )
        .slice()
}

fn use_decl<'a>() -> impl Parser<'a, &'a str, (), Error<'a>> {
    text::keyword("pub")
        .then(text::whitespace().at_least(1))
        .or_not()
        .then(text::keyword("use"))
        .then(text::whitespace().at_least(1))
        .then(text::ident().separated_by(just("::")).at_least(1))
        .then(just(';'))
        .ignored()
}

// Macro that expands `ty` to the type itself _or_ a ref of the type, e.g. u8 or &u8.
// The macro keeps everything as static str.
macro_rules! ty_or_ref {
    ($ty:literal) => {
        just($ty).or(just(concat!('&', $ty)))
    };
}

fn user_ty<'a>(config: &'a Config) -> impl Parser<'a, &'a str, String, Error> + 'a {
    custom(move |input| {
        for (i, ty) in config.user_types.iter().enumerate() {
            let marker = input.save();
            match input.parse(just(ty.parse.as_str())) {
                Ok(_) => {
                    let _ = input.next();
                    return Ok(ty.name.to_string());
                }
                Err(err) => {
                    input.rewind(marker);
                    if i == config.user_types.len() - 1 {
                        return Err(err);
                    }
                }
            }
        }
        // Just need _any error_.
        Err(error::Error::<&'a str>::expected_found(
            None,
            None,
            input.span_since(input.offset()),
        ))
    })
}

fn ty(config: &Config) -> impl Parser<&str, Type, Error> {
    recursive(|nested| {
        choice((
            just("bool").map(|_| Type::Bool),
            ty_or_ref!("u8").map(|_| Type::U8),
            ty_or_ref!("u16").map(|_| Type::U16),
            ty_or_ref!("u32").map(|_| Type::U32),
            ty_or_ref!("u64").map(|_| Type::U64),
            ty_or_ref!("u128").map(|_| Type::U128),
            ty_or_ref!("i8").map(|_| Type::I8),
            ty_or_ref!("i16").map(|_| Type::I16),
            ty_or_ref!("i32").map(|_| Type::I32),
            ty_or_ref!("i64").map(|_| Type::I64),
            ty_or_ref!("i128").map(|_| Type::I128),
            ty_or_ref!("f8").map(|_| Type::F8),
            ty_or_ref!("f16").map(|_| Type::F16),
            ty_or_ref!("f32").map(|_| Type::F32),
            ty_or_ref!("f64").map(|_| Type::F64),
            ty_or_ref!("f128").map(|_| Type::F128),
            ty_or_ref!("String").map(|_| Type::String),
            ty_or_ref!("Vec<u8>").map(|_| Type::Bytes),
            just("&str").map(|_| Type::String),
            just("&[u8]").map(|_| Type::Bytes),
            user_ty(config).map(|name| Type::User(name.to_string())),
            vec(nested.clone()),
            map(nested.clone()),
            option(nested),
            entity_id().map(Type::Api),
        ))
        .boxed()
    })
}

fn vec<'a>(
    ty: impl Parser<'a, &'a str, Type, Error<'a>>,
) -> impl Parser<'a, &'a str, Type, Error<'a>> {
    just("Vec<")
        .then_ignore(whitespace())
        .ignore_then(ty)
        .then_ignore(whitespace())
        .then_ignore(just('>'))
        .map(|inner| Type::new_array(inner))
}

fn map<'a>(
    ty: impl Parser<'a, &'a str, Type, Error<'a>> + Clone,
) -> impl Parser<'a, &'a str, Type, Error<'a>> {
    just("HashMap<")
        .then_ignore(whitespace())
        .ignore_then(ty.clone())
        .then_ignore(just(',').padded())
        .then(ty)
        .then_ignore(just('>'))
        .then_ignore(whitespace())
        .map(|(key, value)| Type::new_map(key, value))
}

fn option<'a>(
    ty: impl Parser<'a, &'a str, Type, Error<'a>>,
) -> impl Parser<'a, &'a str, Type, Error<'a>> {
    just("Option<")
        .then_ignore(whitespace())
        .ignore_then(ty)
        .then_ignore(whitespace())
        .then_ignore(just('>'))
        .map(|inner| Type::new_optional(inner))
}

fn entity_id<'a>() -> impl Parser<'a, &'a str, EntityId, Error<'a>> {
    type_name()
        .separated_by(just("::"))
        .at_least(1)
        .collect::<Vec<_>>()
        .map(|components| EntityId {
            components: components
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<String>>(),
        })
}

fn field<'a>(config: &'a Config) -> impl Parser<'a, &'a str, Field, Error> + 'a {
    text::ident()
        .then_ignore(just(':').padded())
        .then(ty(config))
        .padded()
        .map(|(name, ty)| Field {
            name,
            ty,
            attributes: Default::default(),
        })
        .padded_by(multi_comment())
}

fn dto(config: &Config) -> impl Parser<&str, Dto, Error> {
    let attr = just("#[")
        .then(any().and_is(just("]").not()).repeated().slice())
        .then(just(']'));
    let fields = field(config)
        .separated_by(just(',').padded())
        .allow_trailing()
        .collect::<Vec<_>>()
        .padded_by(multi_comment())
        .delimited_by(just('{').padded(), just('}').padded());
    let name = text::keyword("pub")
        .then(text::whitespace().at_least(1))
        .or_not()
        .ignore_then(text::keyword("struct").padded())
        .ignore_then(text::ident());
    attr.or_not()
        .padded()
        .ignore_then(name)
        .then(fields)
        .map(|(name, fields)| Dto {
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

fn block_comment<'a>() -> impl Parser<'a, &'a str, &'a str, Error<'a>> {
    any()
        .and_is(just("*/").not())
        .repeated()
        .slice()
        .map(&str::trim)
        .delimited_by(just("/*"), just("*/"))
}

fn line_comment<'a>() -> impl Parser<'a, &'a str, &'a str, Error<'a>> {
    just("//").ignore_then(
        any()
            .and_is(just('\n').not())
            .repeated()
            .slice()
            .map(&str::trim),
    )
}

fn comment<'a>() -> impl Parser<'a, &'a str, &'a str, Error<'a>> {
    choice((line_comment(), block_comment()))
}

fn multi_comment<'a>() -> impl Parser<'a, &'a str, Vec<&'a str>, Error<'a>> {
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
    })
}

fn rpc(config: &Config) -> impl Parser<&str, Rpc, Error> {
    let fn_keyword = text::keyword("pub")
        .then(text::whitespace().at_least(1))
        .or_not()
        .then(text::keyword("fn"));
    let name = fn_keyword.padded().ignore_then(text::ident());
    let params = field(config)
        .separated_by(just(',').padded())
        .allow_trailing()
        .collect::<Vec<_>>()
        .padded_by(multi_comment())
        .delimited_by(just('(').padded(), just(')').padded());
    let return_type = just("->").ignore_then(ty(config).padded());
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

const INVALID_ENUM_NUMBER: EnumValueNumber = EnumValueNumber::MAX;
fn en_value<'a>() -> impl Parser<'a, &'a str, EnumValue<'a>, Error<'a>> {
    let number = just('=')
        .padded()
        .ignore_then(text::int(10).try_map(|s, span| {
            str::parse::<EnumValueNumber>(s)
                .map_err(|_| error::Error::<&'a str>::expected_found(None, None, span))
        }));
    text::ident()
        .then(number.or_not())
        .padded()
        .map(|(name, number)| EnumValue {
            name,
            number: number.unwrap_or(INVALID_ENUM_NUMBER),
            attributes: Default::default(),
        })
        .padded_by(multi_comment())
}

fn en<'a>() -> impl Parser<'a, &'a str, Enum<'a>, Error<'a>> {
    let name = text::keyword("pub")
        .then(text::whitespace().at_least(1))
        .or_not()
        .ignore_then(text::keyword("enum").padded())
        .ignore_then(text::ident());
    let values = en_value()
        .separated_by(just(',').padded())
        .allow_trailing()
        .collect::<Vec<_>>()
        .padded_by(multi_comment())
        .delimited_by(just('{').padded(), just('}').padded());
    name.then(values).map(|(name, values)| Enum {
        name,
        values: apply_enum_value_number_defaults(values),
        attributes: Default::default(),
    })
}

fn apply_enum_value_number_defaults(mut values: Vec<EnumValue>) -> Vec<EnumValue> {
    let mut i = 0;
    for value in &mut values {
        if value.number == INVALID_ENUM_NUMBER {
            value.number = i;
            i += 1;
        } else {
            i = value.number + 1;
        }
    }
    values
}

fn namespace_children<'a>(
    config: &'a Config,
    namespace: impl Parser<'a, &'a str, Namespace<'a>, Error<'a>>,
) -> impl Parser<'a, &'a str, Vec<NamespaceChild<'a>>, Error<'a>> {
    choice((
        dto(config).map(NamespaceChild::Dto),
        rpc(config).map(NamespaceChild::Rpc),
        en().map(NamespaceChild::Enum),
        namespace.map(NamespaceChild::Namespace),
    ))
    .padded_by(multi_comment())
    .repeated()
    .collect::<Vec<_>>()
}

fn namespace(config: &Config) -> impl Parser<&str, Namespace, Error> {
    recursive(|nested| {
        let mod_keyword = text::keyword("pub")
            .then(text::whitespace().at_least(1))
            .or_not()
            .then(text::keyword("mod"));
        let body = namespace_children(config, nested)
            .boxed()
            .delimited_by(just('{').padded(), just('}').padded());
        mod_keyword
            .padded()
            .ignore_then(text::ident())
            // or_not to allow declaration-only in the form:
            //      mod name;
            .then(just(';').padded().map(|_| None).or(body.map(|c| Some(c))))
            .map(|(name, children)| Namespace {
                name: Cow::Borrowed(name),
                children: children.unwrap_or(vec![]),
                attributes: Default::default(),
            })
    })
}

#[cfg(test)]
mod tests {
    use anyhow::{anyhow, Result};
    use chumsky::error::Simple;
    use chumsky::Parser;
    use lazy_static::lazy_static;

    use crate::model::{Builder, UNDEFINED_NAMESPACE};
    use crate::parser::rust::field;
    use crate::parser::{Config, UserType};
    use crate::{input, parser, Parser as ApyxlParser};

    type TestError = Vec<Simple<'static, char>>;
    fn wrap_test_err(err: TestError) -> anyhow::Error {
        anyhow!("errors encountered while parsing: {:?}", err)
    }

    lazy_static! {
        static ref CONFIG: Config = Config {
            user_types: vec![UserType {
                parse: "user_type".to_string(),
                name: "user".to_string()
            }]
        };
    }

    #[test]
    fn test_field() -> Result<()> {
        let result = field(&CONFIG).parse("name: Type");
        let output = result.into_result().map_err(wrap_test_err)?;
        assert_eq!(output.name, "name");
        assert_eq!(output.ty.api().unwrap().name().unwrap(), "Type");
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
        fn rpc() {}
        struct dto {}
        mod namespace {}
        "#,
        );
        let mut builder = Builder::default();
        parser::Rust::default().parse(&CONFIG, &mut input, &mut builder)?;
        let model = builder.build().unwrap();
        assert_eq!(model.api().name, UNDEFINED_NAMESPACE);
        assert!(model.api().dto("dto").is_some());
        assert!(model.api().rpc("rpc").is_some());
        assert!(model.api().namespace("namespace").is_some());
        Ok(())
    }

    mod file_path_to_mod {
        use anyhow::Result;

        use crate::model::{Builder, Chunk, EntityId};
        use crate::parser::rust::tests::CONFIG;
        use crate::{input, parser, Parser};

        #[test]
        fn file_path_including_name_without_ext() -> Result<()> {
            let mut input = input::ChunkBuffer::new();
            input.add_chunk(Chunk::with_relative_file_path("a/b/c.rs"), "struct dto {}");
            let mut builder = Builder::default();
            parser::Rust::default().parse(&CONFIG, &mut input, &mut builder)?;
            let model = builder.build().unwrap();

            let namespace = model.api().find_namespace(&EntityId::from("a.b.c"));
            assert!(namespace.is_some());
            assert!(namespace.unwrap().dto("dto").is_some());
            Ok(())
        }

        #[test]
        fn ignore_mod_rs() -> Result<()> {
            let mut input = input::ChunkBuffer::new();
            input.add_chunk(
                Chunk::with_relative_file_path("a/b/mod.rs"),
                "struct dto {}",
            );
            let mut builder = Builder::default();
            parser::Rust::default().parse(&CONFIG, &mut input, &mut builder)?;
            let model = builder.build().unwrap();

            let namespace = model.api().find_namespace(&EntityId::from("a.b"));
            assert!(namespace.is_some());
            assert!(namespace.unwrap().dto("dto").is_some());
            Ok(())
        }

        #[test]
        fn ignore_lib_rs() -> Result<()> {
            let mut input = input::ChunkBuffer::new();
            input.add_chunk(
                Chunk::with_relative_file_path("a/b/lib.rs"),
                "struct dto {}",
            );
            let mut builder = Builder::default();
            parser::Rust::default().parse(&CONFIG, &mut input, &mut builder)?;
            let model = builder.build().unwrap();

            let namespace = model.api().find_namespace(&EntityId::from("a.b"));
            assert!(namespace.is_some());
            assert!(namespace.unwrap().dto("dto").is_some());
            Ok(())
        }
    }

    mod ty {
        use anyhow::Result;
        use chumsky::Parser;

        use crate::model::EntityId;
        use crate::model::Type;
        use crate::parser::rust::tests::wrap_test_err;
        use crate::parser::rust::tests::CONFIG;
        use crate::parser::rust::ty;

        macro_rules! test {
            ($name: ident, $data:literal, $expected:expr) => {
                #[test]
                fn $name() -> Result<()> {
                    run_test($data, $expected)
                }
            };
        }

        test!(bool, "bool", Type::Bool);

        test!(u8, "u8", Type::U8);
        test!(u16, "u16", Type::U16);
        test!(u32, "u32", Type::U32);
        test!(u64, "u64", Type::U64);
        test!(u128, "u128", Type::U128);
        test!(i8, "i8", Type::I8);
        test!(i16, "i16", Type::I16);
        test!(i32, "i32", Type::I32);
        test!(i64, "i64", Type::I64);
        test!(i128, "i128", Type::I128);
        test!(f8, "f8", Type::F8);
        test!(f16, "f16", Type::F16);
        test!(f32, "f32", Type::F32);
        test!(f64, "f64", Type::F64);
        test!(f128, "f128", Type::F128);
        test!(string, "String", Type::String);
        test!(bytes, "Vec<u8>", Type::Bytes);

        test!(u8_ref, "&u8", Type::U8);
        test!(u16_ref, "&u16", Type::U16);
        test!(u32_ref, "&u32", Type::U32);
        test!(u64_ref, "&u64", Type::U64);
        test!(u128_ref, "&u128", Type::U128);
        test!(i8_ref, "&i8", Type::I8);
        test!(i16_ref, "&i16", Type::I16);
        test!(i32_ref, "&i32", Type::I32);
        test!(i64_ref, "&i64", Type::I64);
        test!(i128_ref, "&i128", Type::I128);
        test!(f8_ref, "&f8", Type::F8);
        test!(f16_ref, "&f16", Type::F16);
        test!(f32_ref, "&f32", Type::F32);
        test!(f64_ref, "&f64", Type::F64);
        test!(f128_ref, "&f128", Type::F128);
        test!(string_ref, "&String", Type::String);
        test!(bytes_ref, "&Vec<u8>", Type::Bytes);

        test!(str, "&str", Type::String);
        test!(bytes_slice, "&[u8]", Type::Bytes);
        test!(entity_id, "a::b::c", Type::Api(EntityId::from("a.b.c")));

        // Vec/Array.
        test!(vec, "Vec<i32>", Type::new_array(Type::I32));
        test!(
            vec_api,
            "Vec<a::b::c>",
            Type::new_array(Type::Api(EntityId::from("a.b.c")))
        );
        test!(
            vec_nested,
            "Vec<Vec<Vec<String>>>",
            Type::new_array(Type::new_array(Type::new_array(Type::String)))
        );

        // Map.
        test!(
            map,
            "HashMap<String, i32>",
            Type::new_map(Type::String, Type::I32)
        );
        test!(
            map_api,
            "HashMap<dto, a::b::c>",
            Type::new_map(
                Type::Api(EntityId::from("dto")),
                Type::Api(EntityId::from("a.b.c")),
            )
        );
        test!(
            map_nested,
            "HashMap<String, HashMap<HashMap<i32, f32>, String>>",
            Type::new_map(
                Type::String,
                Type::new_map(Type::new_map(Type::I32, Type::F32), Type::String)
            )
        );

        // Option.
        test!(option, "Option<i32>", Type::Optional(Box::new(Type::I32)));
        test!(
            option_api,
            "Option<a::b::c>",
            Type::new_optional(Type::Api(EntityId::from("a.b.c")))
        );
        test!(
            option_nested,
            "Option<Option<Option<String>>>",
            Type::new_optional(Type::new_optional(Type::new_optional(Type::String)))
        );

        // Combined complex types.
        test!(
            complex_nested,
            "HashMap<Option<String>, Vec<String>>",
            Type::new_map(
                Type::new_optional(Type::String),
                Type::new_array(Type::String),
            )
        );

        // Defined in CONFIG.
        test!(user, "user_type", Type::User("user".to_string()));

        fn run_test(data: &'static str, expected: Type) -> Result<()> {
            let ty = ty(&CONFIG)
                .parse(data)
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(ty, expected);
            Ok(())
        }
    }

    mod user_ty {
        use chumsky::Parser;

        use crate::parser::rust::user_ty;
        use crate::parser::{Config, UserType};

        #[test]
        fn test() {
            let config = Config {
                user_types: vec![
                    UserType {
                        parse: "i32".to_string(),
                        name: "int".to_string(),
                    },
                    UserType {
                        parse: "f32".to_string(),
                        name: "float".to_string(),
                    },
                ],
            };
            let ty = user_ty(&config).parse("i32").into_output().unwrap();
            assert_eq!(ty, "int");
            let ty = user_ty(&config).parse("f32").into_output().unwrap();
            assert_eq!(ty, "float");
        }
    }

    mod entity_id {
        use anyhow::Result;
        use chumsky::Parser;

        use crate::parser::rust::entity_id;
        use crate::parser::rust::tests::wrap_test_err;

        #[test]
        fn starts_with_underscore() -> Result<()> {
            let id = entity_id()
                .parse("_type")
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(id.path, vec!["_type"]);
            Ok(())
        }

        #[test]
        fn with_path() -> Result<()> {
            let id = entity_id()
                .parse("a::b::c")
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(id.path, vec!["a", "b", "c"]);
            Ok(())
        }

        #[test]
        fn reference() -> Result<()> {
            let id = entity_id()
                .parse("&Type")
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(id.path, vec!["&Type"]);
            Ok(())
        }
    }

    mod namespace {
        use anyhow::Result;
        use chumsky::Parser;

        use crate::model::NamespaceChild;
        use crate::parser::rust::namespace;
        use crate::parser::rust::tests::wrap_test_err;
        use crate::parser::rust::tests::CONFIG;

        #[test]
        fn declaration() -> Result<()> {
            let namespace = namespace(&CONFIG)
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
        fn empty() -> Result<()> {
            let namespace = namespace(&CONFIG)
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
            let namespace = namespace(&CONFIG)
                .parse(
                    r#"
            mod ns {
                struct DtoName {}
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
            let namespace = namespace(&CONFIG)
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
            let namespace = namespace(&CONFIG)
                .parse(
                    r#"
            mod ns0 {
                mod ns1 {
                    struct DtoName {}
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
    }

    mod dto {
        use anyhow::Result;
        use chumsky::Parser;

        use crate::parser::rust::dto;
        use crate::parser::rust::tests::wrap_test_err;
        use crate::parser::rust::tests::CONFIG;

        #[test]
        fn empty() -> Result<()> {
            let dto = dto(&CONFIG)
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
        fn pub_struct() -> Result<()> {
            let dto = dto(&CONFIG)
                .parse(
                    r#"
            pub struct StructName {}
            "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(dto.name, "StructName");
            assert_eq!(dto.fields.len(), 0);
            Ok(())
        }

        #[test]
        fn ignore_derive() -> Result<()> {
            let dto = dto(&CONFIG)
                .parse(
                    r#"
            #[derive(Whatever)]
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
            let dto = dto(&CONFIG)
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
        fn fields_with_comments() -> Result<()> {
            let dto = dto(&CONFIG)
                .parse(
                    r#"
            struct StructName {
                // asdf
                // asdf
                field0: i32, /* asdf */ field1: f32,
                // asdf
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
    }

    mod rpc {
        use anyhow::Result;
        use chumsky::Parser;

        use crate::parser::rust::rpc;
        use crate::parser::rust::tests::wrap_test_err;
        use crate::parser::rust::tests::CONFIG;

        #[test]
        fn empty_fn() -> Result<()> {
            let rpc = rpc(&CONFIG)
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
        fn pub_fn() -> Result<()> {
            let rpc = rpc(&CONFIG)
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
            Ok(())
        }

        #[test]
        fn fn_keyword_smushed() {
            let rpc = rpc(&CONFIG)
                .parse(
                    r#"
            pubfn rpc_name() {}
            "#,
                )
                .into_result();
            assert!(rpc.is_err());
        }

        #[test]
        fn single_param() -> Result<()> {
            let rpc = rpc(&CONFIG)
                .parse(
                    r#"
            fn rpc_name(param0: ParamType0) {}
            "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(rpc.params.len(), 1);
            assert_eq!(rpc.params[0].name, "param0");
            assert_eq!(rpc.params[0].ty.api().unwrap().name(), Some("ParamType0"));
            Ok(())
        }

        #[test]
        fn multiple_params() -> Result<()> {
            let rpc = rpc(&CONFIG)
                .parse(
                    r#"
            fn rpc_name(param0: ParamType0, param1: ParamType1, param2: ParamType2) {}
            "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(rpc.params.len(), 3);
            assert_eq!(rpc.params[0].name, "param0");
            assert_eq!(rpc.params[0].ty.api().unwrap().name(), Some("ParamType0"));
            assert_eq!(rpc.params[1].name, "param1");
            assert_eq!(rpc.params[1].ty.api().unwrap().name(), Some("ParamType1"));
            assert_eq!(rpc.params[2].name, "param2");
            assert_eq!(rpc.params[2].ty.api().unwrap().name(), Some("ParamType2"));
            Ok(())
        }

        #[test]
        fn multiple_params_with_comments() -> Result<()> {
            let rpc = rpc(&CONFIG)
                .parse(
                    r#"
            fn rpc_name(
                // asdf
                // asdf
                param0: ParamType0, /* asdf */ param1: ParamType1,
                // asdf
                param2: ParamType2 /* asdf */
                // asdf
            ) {}
            "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(rpc.params.len(), 3);
            assert_eq!(rpc.params[0].name, "param0");
            assert_eq!(rpc.params[0].ty.api().unwrap().name(), Some("ParamType0"));
            assert_eq!(rpc.params[1].name, "param1");
            assert_eq!(rpc.params[1].ty.api().unwrap().name(), Some("ParamType1"));
            assert_eq!(rpc.params[2].name, "param2");
            assert_eq!(rpc.params[2].ty.api().unwrap().name(), Some("ParamType2"));
            Ok(())
        }

        #[test]
        fn multiple_params_weird_spacing_trailing_comma() -> Result<()> {
            let rpc = rpc(&CONFIG)
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
            assert_eq!(rpc.params[0].ty.api().unwrap().name(), Some("ParamType0"));
            assert_eq!(rpc.params[1].name, "param1");
            assert_eq!(rpc.params[1].ty.api().unwrap().name(), Some("ParamType1"));
            assert_eq!(rpc.params[2].name, "param2");
            assert_eq!(rpc.params[2].ty.api().unwrap().name(), Some("ParamType2"));
            Ok(())
        }

        #[test]
        fn return_type() -> Result<()> {
            let rpc = rpc(&CONFIG)
                .parse(
                    r#"
            fn rpc_name() -> Asdfg {}
            "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(
                rpc.return_type.as_ref().map(|x| x.api().unwrap().name()),
                Some(Some("Asdfg"))
            );
            Ok(())
        }

        #[test]
        fn return_type_weird_spacing() -> Result<()> {
            let rpc = rpc(&CONFIG)
                .parse(
                    r#"
            fn rpc_name()           ->Asdfg{}
            "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(
                rpc.return_type.as_ref().map(|x| x.api().unwrap().name()),
                Some(Some("Asdfg"))
            );
            Ok(())
        }
    }

    mod en_value {
        use anyhow::Result;
        use chumsky::Parser;

        use crate::parser::rust::en_value;
        use crate::parser::rust::tests::wrap_test_err;

        #[test]
        fn test() -> Result<()> {
            let value = en_value()
                .parse("Value = 1")
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(value.name, "Value");
            assert_eq!(value.number, 1);
            Ok(())
        }
    }

    mod en {
        use anyhow::Result;
        use chumsky::Parser;

        use crate::model::{EnumValue, EnumValueNumber};
        use crate::parser::rust::en;
        use crate::parser::rust::tests::wrap_test_err;

        #[test]
        fn without_numbers() -> Result<()> {
            let en = en()
                .parse(
                    r#"
                    enum en {
                        Value0,
                        Value1,
                        Value2,
                    }
                "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(en.name, "en");
            assert_value(en.values.get(0), "Value0", 0);
            assert_value(en.values.get(1), "Value1", 1);
            assert_value(en.values.get(2), "Value2", 2);
            Ok(())
        }

        #[test]
        fn with_numbers() -> Result<()> {
            let en = en()
                .parse(
                    r#"
                    enum en {
                        Value0 = 10,
                        Value1 = 25,
                        Value2 = 999,
                        SameNum = 999,
                    }
                "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(en.name, "en");
            assert_value(en.values.get(0), "Value0", 10);
            assert_value(en.values.get(1), "Value1", 25);
            assert_value(en.values.get(2), "Value2", 999);
            assert_value(en.values.get(3), "SameNum", 999);
            Ok(())
        }

        #[test]
        fn with_mixed_numbers() -> Result<()> {
            let en = en()
                .parse(
                    r#"
                    enum en {
                        Value0,
                        Value1 = 25,
                        Value2,
                        SameNum = 999,
                    }
                "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(en.name, "en");
            assert_value(en.values.get(0), "Value0", 0);
            assert_value(en.values.get(1), "Value1", 25);
            assert_value(en.values.get(2), "Value2", 26);
            assert_value(en.values.get(3), "SameNum", 999);
            Ok(())
        }

        fn assert_value(
            actual: Option<&EnumValue>,
            expected_name: &str,
            expected_number: EnumValueNumber,
        ) {
            assert_eq!(
                actual,
                Some(&EnumValue {
                    name: expected_name,
                    number: expected_number,
                    ..Default::default()
                })
            );
        }
    }

    mod comments {
        use anyhow::Result;
        use chumsky::Parser;

        use crate::parser::rust::tests::wrap_test_err;
        use crate::parser::rust::tests::CONFIG;
        use crate::parser::rust::{comment, namespace};

        #[test]
        fn line_comment() -> Result<()> {
            let value = comment()
                .parse("// line comment")
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(value, "line comment");
            Ok(())
        }

        #[test]
        fn block_comment() -> Result<()> {
            let value = comment()
                .parse("/* block comment */")
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(value, "block comment");
            Ok(())
        }

        #[test]
        fn line_comment_inside_namespace() -> Result<()> {
            namespace(&CONFIG)
                .parse(
                    r#"
                    mod ns { // comment
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
            namespace(&CONFIG)
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
