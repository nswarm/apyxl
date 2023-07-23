use chumsky::prelude::{any, choice, custom, just, recursive};
use chumsky::{error, text, IterParser, Parser};

use crate::model::{EntityId, Type};
use crate::parser::rust::Error;
use crate::parser::Config;

const ALLOWED_TYPE_NAME_CHARS: &str = "_&<>";

// Macro that expands `ty` to the type itself _or_ a ref of the type, e.g. u8 or &u8.
// The macro keeps everything as static str.
macro_rules! ty_or_ref {
    ($ty:literal) => {
        just($ty).or(just(concat!('&', $ty)))
    };
}

pub fn ty(config: &Config) -> impl Parser<&str, Type, Error> {
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
            user_ty(config).map(Type::User),
            vec(nested.clone()),
            map(nested.clone()),
            option(nested),
            entity_id().map(Type::Api),
        ))
        .boxed()
    })
}

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

fn vec<'a>(
    ty: impl Parser<'a, &'a str, Type, Error<'a>>,
) -> impl Parser<'a, &'a str, Type, Error<'a>> {
    just("Vec<")
        .then_ignore(text::whitespace())
        .ignore_then(ty)
        .then_ignore(text::whitespace())
        .then_ignore(just('>'))
        .map(|inner| Type::new_array(inner))
}

fn map<'a>(
    ty: impl Parser<'a, &'a str, Type, Error<'a>> + Clone,
) -> impl Parser<'a, &'a str, Type, Error<'a>> {
    just("HashMap<")
        .then_ignore(text::whitespace())
        .ignore_then(ty.clone())
        .then_ignore(just(',').padded())
        .then(ty)
        .then_ignore(just('>'))
        .then_ignore(text::whitespace())
        .map(|(key, value)| Type::new_map(key, value))
}

fn option<'a>(
    ty: impl Parser<'a, &'a str, Type, Error<'a>>,
) -> impl Parser<'a, &'a str, Type, Error<'a>> {
    just("Option<")
        .then_ignore(text::whitespace())
        .ignore_then(ty)
        .then_ignore(text::whitespace())
        .then_ignore(just('>'))
        .map(Type::new_optional)
}

fn user_ty(config: &Config) -> impl Parser<&str, String, Error> {
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
        Err(error::Error::<&str>::expected_found(
            None,
            None,
            input.span_since(input.offset()),
        ))
    })
}

fn entity_id<'a>() -> impl Parser<'a, &'a str, EntityId, Error<'a>> {
    type_name()
        .separated_by(just("::"))
        .at_least(1)
        .collect::<Vec<_>>()
        .map(|components| EntityId::new_unqualified_vec(components.into_iter()))
}

#[cfg(test)]
mod tests {
    mod ty {
        use anyhow::Result;
        use chumsky::Parser;
        use lazy_static::lazy_static;

        use crate::model::EntityId;
        use crate::model::Type;
        use crate::parser::rust::tests::wrap_test_err;
        use crate::parser::rust::ty;
        use crate::parser::{Config, UserType};

        lazy_static! {
            static ref TY_TEST_CONFIG: Config = Config {
                user_types: vec![UserType {
                    parse: "user_type".to_string(),
                    name: "user".to_string()
                }],
                enable_parse_private: true,
            };
        }

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
        test!(
            entity_id,
            "a::b::c",
            Type::Api(EntityId::new_unqualified("a.b.c"))
        );

        // Vec/Array.
        test!(vec, "Vec<i32>", Type::new_array(Type::I32));
        test!(
            vec_api,
            "Vec<a::b::c>",
            Type::new_array(Type::Api(EntityId::new_unqualified("a.b.c")))
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
                Type::Api(EntityId::new_unqualified("dto")),
                Type::Api(EntityId::new_unqualified("a.b.c")),
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
            Type::new_optional(Type::Api(EntityId::new_unqualified("a.b.c")))
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
            let ty = ty::ty(&TY_TEST_CONFIG)
                .parse(data)
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(ty, expected);
            Ok(())
        }
    }

    mod user_ty {
        use chumsky::Parser;

        use crate::parser::rust::ty::user_ty;
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
                ..Default::default()
            };
            let ty = user_ty(&config).parse("i32").into_output().unwrap();
            assert_eq!(ty, "int");
            let ty = user_ty(&config).parse("f32").into_output().unwrap();
            assert_eq!(ty, "float");
        }
    }

    mod entity_id {
        use crate::parser::rust::tests::wrap_test_err;
        use crate::parser::rust::ty::entity_id;
        use anyhow::Result;
        use chumsky::Parser;
        use itertools::Itertools;

        #[test]
        fn starts_with_underscore() -> Result<()> {
            let id = entity_id()
                .parse("_type")
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(id.component_names().collect_vec(), vec!["_type"]);
            Ok(())
        }

        #[test]
        fn with_path() -> Result<()> {
            let id = entity_id()
                .parse("a::b::c")
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(id.component_names().collect_vec(), vec!["a", "b", "c"]);
            Ok(())
        }

        #[test]
        fn reference() -> Result<()> {
            let id = entity_id()
                .parse("&Type")
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(id.component_names().collect_vec(), vec!["&Type"]);
            Ok(())
        }
    }
}
