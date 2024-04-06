use chumsky::prelude::*;

use crate::model::{EntityId, Semantics, Type, TypeRef};
use crate::parser::error::Error;
use crate::parser::Config;

const ALLOWED_TYPE_NAME_CHARS: &str = "_<>";

pub fn parser(config: &Config) -> impl Parser<&str, TypeRef, Error> {
    let ref_type = choice((
        just("&mut")
            .then(text::whitespace().at_least(1))
            .map(|_| Semantics::Mut),
        just("&").map(|_| Semantics::Ref),
    ));
    recursive(|nested| {
        let ty = choice((
            just("bool").map(|_| Type::Bool),
            just("u8").map(|_| Type::U8),
            just("u16").map(|_| Type::U16),
            just("u32").map(|_| Type::U32),
            just("u64").map(|_| Type::U64),
            just("u128").map(|_| Type::U128),
            just("usize").map(|_| Type::USIZE),
            just("i8").map(|_| Type::I8),
            just("i16").map(|_| Type::I16),
            just("i32").map(|_| Type::I32),
            just("i64").map(|_| Type::I64),
            just("i128").map(|_| Type::I128),
            just("f8").map(|_| Type::F8),
            just("f16").map(|_| Type::F16),
            just("f32").map(|_| Type::F32),
            just("f64").map(|_| Type::F64),
            just("f128").map(|_| Type::F128),
            just("String").map(|_| Type::String),
            just("Vec<u8>").map(|_| Type::Bytes),
            just("str").map(|_| Type::String),
            just("String").map(|_| Type::String),
            just("[u8]").map(|_| Type::Bytes),
        ))
        .or(choice((
            user_ty(config).map(Type::User),
            vec(nested.clone()),
            map(nested.clone()),
            option(nested),
            // Note that entity_id should come last because it is greedy.
            entity_id().map(Type::Api),
        )))
        .boxed();
        ref_type
            .or_not()
            .then(ty)
            .map(|(semantics, ty)| TypeRef {
                value: ty,
                semantics: semantics.unwrap_or(Semantics::Value),
            })
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
    ty: impl Parser<'a, &'a str, TypeRef, Error<'a>>,
) -> impl Parser<'a, &'a str, Type, Error<'a>> {
    just("Vec<")
        .then_ignore(text::whitespace())
        .ignore_then(ty)
        .then_ignore(text::whitespace())
        .then_ignore(just('>'))
        .map(Type::new_array)
}

fn map<'a>(
    ty: impl Parser<'a, &'a str, TypeRef, Error<'a>> + Clone,
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
    ty: impl Parser<'a, &'a str, TypeRef, Error<'a>>,
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
        Err(chumsky::error::Error::<&str>::expected_found(
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

        use crate::model::{EntityId, Semantics, Type, TypeRef};
        use crate::parser::rust::ty;
        use crate::parser::test_util::wrap_test_err;
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

        test!(bool, "bool", TypeRef::new(Type::Bool, Semantics::Value));

        test!(u8, "u8", TypeRef::new(Type::U8, Semantics::Value));
        test!(u16, "u16", TypeRef::new(Type::U16, Semantics::Value));
        test!(u32, "u32", TypeRef::new(Type::U32, Semantics::Value));
        test!(u64, "u64", TypeRef::new(Type::U64, Semantics::Value));
        test!(u128, "u128", TypeRef::new(Type::U128, Semantics::Value));
        test!(usize, "usize", TypeRef::new(Type::USIZE, Semantics::Value));
        test!(i8, "i8", TypeRef::new(Type::I8, Semantics::Value));
        test!(i16, "i16", TypeRef::new(Type::I16, Semantics::Value));
        test!(i32, "i32", TypeRef::new(Type::I32, Semantics::Value));
        test!(i64, "i64", TypeRef::new(Type::I64, Semantics::Value));
        test!(i128, "i128", TypeRef::new(Type::I128, Semantics::Value));
        test!(f8, "f8", TypeRef::new(Type::F8, Semantics::Value));
        test!(f16, "f16", TypeRef::new(Type::F16, Semantics::Value));
        test!(f32, "f32", TypeRef::new(Type::F32, Semantics::Value));
        test!(f64, "f64", TypeRef::new(Type::F64, Semantics::Value));
        test!(f128, "f128", TypeRef::new(Type::F128, Semantics::Value));
        test!(
            string,
            "String",
            TypeRef::new(Type::String, Semantics::Value)
        );
        test!(
            bytes,
            "Vec<u8>",
            TypeRef::new(Type::Bytes, Semantics::Value)
        );

        test!(u8_ref, "&u8", TypeRef::new(Type::U8, Semantics::Ref));
        test!(u16_ref, "&u16", TypeRef::new(Type::U16, Semantics::Ref));
        test!(u32_ref, "&u32", TypeRef::new(Type::U32, Semantics::Ref));
        test!(u64_ref, "&u64", TypeRef::new(Type::U64, Semantics::Ref));
        test!(u128_ref, "&u128", TypeRef::new(Type::U128, Semantics::Ref));
        test!(
            usize_ref,
            "&usize",
            TypeRef::new(Type::USIZE, Semantics::Ref)
        );
        test!(i8_ref, "&i8", TypeRef::new(Type::I8, Semantics::Ref));
        test!(i16_ref, "&i16", TypeRef::new(Type::I16, Semantics::Ref));
        test!(i32_ref, "&i32", TypeRef::new(Type::I32, Semantics::Ref));
        test!(i64_ref, "&i64", TypeRef::new(Type::I64, Semantics::Ref));
        test!(i128_ref, "&i128", TypeRef::new(Type::I128, Semantics::Ref));
        test!(f8_ref, "&f8", TypeRef::new(Type::F8, Semantics::Ref));
        test!(f16_ref, "&f16", TypeRef::new(Type::F16, Semantics::Ref));
        test!(f32_ref, "&f32", TypeRef::new(Type::F32, Semantics::Ref));
        test!(f64_ref, "&f64", TypeRef::new(Type::F64, Semantics::Ref));
        test!(f128_ref, "&f128", TypeRef::new(Type::F128, Semantics::Ref));
        test!(
            string_ref,
            "&String",
            TypeRef::new(Type::String, Semantics::Ref)
        );
        test!(
            bytes_ref,
            "&Vec<u8>",
            TypeRef::new(Type::Bytes, Semantics::Ref)
        );

        test!(
            string_mut,
            "&mut String",
            TypeRef::new(Type::String, Semantics::Mut)
        );
        test!(
            bytes_mut,
            "&mut Vec<u8>",
            TypeRef::new(Type::Bytes, Semantics::Mut)
        );

        test!(str, "&str", TypeRef::new(Type::String, Semantics::Ref));
        test!(
            bytes_slice,
            "&[u8]",
            TypeRef::new(Type::Bytes, Semantics::Ref)
        );
        test!(
            entity_id,
            "a::b::c",
            TypeRef::new(
                Type::Api(EntityId::new_unqualified("a.b.c")),
                Semantics::Value
            )
        );
        test!(
            entity_id_ref,
            "&a::b::c",
            TypeRef::new(
                Type::Api(EntityId::new_unqualified("a.b.c")),
                Semantics::Ref
            )
        );
        test!(
            entity_id_mut,
            "&mut a::b::c",
            TypeRef::new(
                Type::Api(EntityId::new_unqualified("a.b.c")),
                Semantics::Mut
            )
        );

        // Vec/Array.
        test!(
            vec,
            "Vec<i32>",
            TypeRef::new(
                Type::new_array(TypeRef::new(Type::I32, Semantics::Value)),
                Semantics::Value
            )
        );
        test!(
            vec_api,
            "Vec<a::b::c>",
            TypeRef::new(
                Type::new_array(TypeRef::new(
                    Type::Api(EntityId::new_unqualified("a.b.c")),
                    Semantics::Value
                )),
                Semantics::Value
            )
        );
        test!(
            vec_nested,
            "Vec<Vec<Vec<String>>>",
            TypeRef::new(
                Type::new_array(TypeRef::new(
                    Type::new_array(TypeRef::new(
                        Type::new_array(TypeRef::new(Type::String, Semantics::Value)),
                        Semantics::Value
                    )),
                    Semantics::Value
                )),
                Semantics::Value
            )
        );

        // Map.
        test!(
            map,
            "HashMap<String, i32>",
            TypeRef::new(
                Type::new_map(
                    TypeRef::new(Type::String, Semantics::Value),
                    TypeRef::new(Type::I32, Semantics::Value)
                ),
                Semantics::Value
            )
        );
        test!(
            map_api,
            "HashMap<dto, a::b::c>",
            TypeRef::new(
                Type::new_map(
                    TypeRef::new(
                        Type::Api(EntityId::new_unqualified("dto")),
                        Semantics::Value
                    ),
                    TypeRef::new(
                        Type::Api(EntityId::new_unqualified("a.b.c")),
                        Semantics::Value
                    ),
                ),
                Semantics::Value
            )
        );
        test!(
            map_nested,
            "HashMap<String, HashMap<HashMap<i32, f32>, String>>",
            TypeRef::new(
                Type::new_map(
                    TypeRef::new(Type::String, Semantics::Value),
                    TypeRef::new(
                        Type::new_map(
                            TypeRef::new(
                                Type::new_map(
                                    TypeRef::new(Type::I32, Semantics::Value),
                                    TypeRef::new(Type::F32, Semantics::Value)
                                ),
                                Semantics::Value
                            ),
                            TypeRef::new(Type::String, Semantics::Value),
                        ),
                        Semantics::Value
                    )
                ),
                Semantics::Value
            )
        );

        // Option.
        test!(
            option,
            "Option<i32>",
            TypeRef::new(
                Type::Optional(Box::new(TypeRef::new(Type::I32, Semantics::Value))),
                Semantics::Value
            )
        );
        test!(
            option_api,
            "Option<a::b::c>",
            TypeRef::new(
                Type::new_optional(TypeRef::new(
                    Type::Api(EntityId::new_unqualified("a.b.c")),
                    Semantics::Value
                )),
                Semantics::Value
            )
        );
        test!(
            option_nested,
            "Option<Option<Option<String>>>",
            TypeRef::new(
                Type::new_optional(TypeRef::new(
                    Type::new_optional(TypeRef::new(
                        Type::new_optional(TypeRef::new(Type::String, Semantics::Value)),
                        Semantics::Value
                    )),
                    Semantics::Value
                )),
                Semantics::Value
            )
        );

        // Combined complex types.
        test!(
            complex_nested,
            "HashMap<Option<String>, Vec<String>>",
            TypeRef::new(
                Type::new_map(
                    TypeRef::new(
                        Type::new_optional(TypeRef::new(Type::String, Semantics::Value)),
                        Semantics::Value
                    ),
                    TypeRef::new(
                        Type::new_array(TypeRef::new(Type::String, Semantics::Value)),
                        Semantics::Value
                    ),
                ),
                Semantics::Value
            )
        );

        test!(
            complex_nested_refs,
            "HashMap<&Option<&mut String>, &mut Vec<&String>>",
            TypeRef::new_map(
                TypeRef::new_optional(TypeRef::new(Type::String, Semantics::Mut), Semantics::Ref),
                TypeRef::new_array(TypeRef::new(Type::String, Semantics::Ref), Semantics::Mut),
                Semantics::Value
            )
        );

        // Defined in CONFIG.
        test!(
            user,
            "user_type",
            TypeRef::new(Type::User("user".to_string()), Semantics::Value)
        );

        fn run_test(data: &'static str, expected: TypeRef) -> Result<()> {
            let ty = ty::parser(&TY_TEST_CONFIG)
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
        use anyhow::Result;
        use chumsky::Parser;
        use itertools::Itertools;

        use crate::parser::rust::ty::entity_id;
        use crate::parser::test_util::wrap_test_err;

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
        fn basic() -> Result<()> {
            let id = entity_id()
                .parse("Type")
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(id.component_names().collect_vec(), vec!["Type"]);
            Ok(())
        }
    }
}
