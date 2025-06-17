use chumsky::input::InputRef;
use chumsky::prelude::*;

use apyxl::model::{EntityId, Semantics, Type, TypeRef};
use apyxl::parser::error::Error;
use apyxl::parser::Config;

const ALLOWED_TYPE_NAME_CHARS: &str = "_<>";

pub fn parser(config: &Config) -> impl Parser<&str, TypeRef, Error> {
    recursive(|nested| {
        let optional_parser = optional(config, nested.clone());
        let array_parser = array(config, nested.clone(), optional_parser.clone());
        ty(config, nested, array_parser, optional_parser).boxed()
    })
}

fn ty<'a>(
    config: &'a Config,
    nested: impl Parser<'a, &'a str, TypeRef, Error<'a>> + Clone + 'a,
    array: impl Parser<'a, &'a str, Type, Error<'a>> + Clone + 'a,
    optional: impl Parser<'a, &'a str, Type, Error<'a>> + Clone + 'a,
) -> impl Parser<'a, &'a str, TypeRef, Error<'a>> + Clone {
    let ty = choice((
        // Array/optional must be first to properly catch primitive types since they use type suffixes.
        array,
        optional,
        just("byte[]").map(|_| Type::Bytes),
        just("bool").map(|_| Type::Bool),
        just("byte").map(|_| Type::U8),
        just("ushort").map(|_| Type::U16),
        just("uint").map(|_| Type::U32),
        just("ulong").map(|_| Type::U64),
        just("sbyte").map(|_| Type::I8),
        just("short").map(|_| Type::I16),
        just("int").map(|_| Type::I32),
        just("long").map(|_| Type::I64),
        just("float").map(|_| Type::F32),
        just("double").map(|_| Type::F64),
        just("string").map(|_| Type::String),
    ))
    .or(choice((
        user_ty(config).map(Type::User),
        list(nested.clone()),
        map(nested.clone()),
        entity_id().map(Type::Api),
    )))
    .boxed();
    ty.map(|ty| TypeRef {
        value: ty,
        semantics: Semantics::Value,
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

fn list<'a>(
    ty: impl Parser<'a, &'a str, TypeRef, Error<'a>>,
) -> impl Parser<'a, &'a str, Type, Error<'a>> {
    just("List<")
        .ignore_then(ty.padded())
        .then_ignore(just('>'))
        .map(Type::new_array)
}

fn array<'a>(
    config: &'a Config,
    nested_ty: impl Parser<'a, &'a str, TypeRef, Error<'a>> + Clone + 'a,
    optional: impl Parser<'a, &'a str, Type, Error<'a>> + Clone + 'a,
) -> impl Parser<'a, &'a str, Type, Error<'a>> + Clone {
    custom(move |input: &mut InputRef<&'a str, Error<'a>>| {
        // Instead of trying to do left recursion shenanigans already inside a recursive
        // parser, use lookahead to see how many nested arrays there are, parse the type as
        // a ty that _cannot be an array_, then iteratively wrap it in array Types.
        //
        // (note that it can still have arrays deeper
        // which is why we still pass it the main ty parser).
        let array_ty = ty(
            config,
            nested_ty.clone(),
            error_ty_parser(),
            optional.clone(),
        );
        let lookahead = any()
            .filter(|c: &char| c.is_alphanumeric() || "._<>?".contains(*c))
            .repeated()
            .at_least(1)
            .slice()
            .ignore_then(just("[]").padded().repeated().at_least(1).count());

        let marker = input.save();
        let depth = match input.parse(lookahead) {
            Ok(count) => count,
            Err(err) => return Err(err),
        };
        input.rewind(marker);

        let parser = array_ty
            .clone()
            .then_ignore(just("[]").repeated().exactly(depth).padded());
        let array_ty = input.parse(parser)?;

        let mut return_ty = Type::new_array(array_ty);
        for _ in 0..(depth - 1) {
            return_ty = Type::new_array(TypeRef::new(return_ty, Semantics::Value));
        }
        Ok(return_ty)
    })
}

fn optional<'a>(
    config: &'a Config,
    nested_ty: impl Parser<'a, &'a str, TypeRef, Error<'a>> + Clone + 'a,
) -> impl Parser<'a, &'a str, Type, Error<'a>> + Clone {
    // Not allowing arbitrary type arrays that are optional (e.g. int[]?) because
    // 1. That seems unlikely (just use List)
    // 2. I don't have a good answer to the mutual recursion problem.
    ty(config, nested_ty, error_ty_parser(), error_ty_parser())
        .then_ignore(just('?').padded())
        .map(Type::new_optional)
}

/// Parser that always returns an error. Use for dynamically skipping choices.
fn error_ty_parser<'a>() -> impl Parser<'a, &'a str, Type, Error<'a>> + Clone {
    empty().try_map(|_, span| Err(Rich::custom(span, "will never show")))
}

fn map<'a>(
    ty: impl Parser<'a, &'a str, TypeRef, Error<'a>> + Clone,
) -> impl Parser<'a, &'a str, Type, Error<'a>> {
    just("Dictionary<")
        .then_ignore(text::whitespace())
        .ignore_then(ty.clone())
        .then_ignore(just(',').padded())
        .then(ty)
        .then_ignore(just('>'))
        .then_ignore(text::whitespace())
        .map(|(key, value)| Type::new_map(key, value))
}

// fn option<'a>(
//     ty: impl Parser<'a, &'a str, TypeRef, Error<'a>>,
// ) -> impl Parser<'a, &'a str, Type, Error<'a>> {
//     ty.then_ignore(just('?')).map(Type::new_optional)
// }

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
        .separated_by(just("."))
        .at_least(1)
        .collect::<Vec<_>>()
        .map(|components| EntityId::new_unqualified_vec(components.into_iter()))
}

#[cfg(test)]
mod tests {
    use chumsky::Parser;

    mod ty {
        use anyhow::Result;
        use chumsky::Parser;
        use lazy_static::lazy_static;

        use crate::parser::ty;
        use apyxl::model::{EntityId, Semantics, Type, TypeRef};
        use apyxl::parser::test_util::wrap_test_err;
        use apyxl::parser::{Config, UserType};

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

        test!(byte, "byte", TypeRef::new(Type::U8, Semantics::Value));
        test!(ushort, "ushort", TypeRef::new(Type::U16, Semantics::Value));
        test!(uint, "uint", TypeRef::new(Type::U32, Semantics::Value));
        test!(ulong, "ulong", TypeRef::new(Type::U64, Semantics::Value));
        test!(sbyte, "sbyte", TypeRef::new(Type::I8, Semantics::Value));
        test!(short, "short", TypeRef::new(Type::I16, Semantics::Value));
        test!(int, "int", TypeRef::new(Type::I32, Semantics::Value));
        test!(long, "long", TypeRef::new(Type::I64, Semantics::Value));
        test!(float, "float", TypeRef::new(Type::F32, Semantics::Value));
        test!(double, "double", TypeRef::new(Type::F64, Semantics::Value));
        test!(
            string,
            "string",
            TypeRef::new(Type::String, Semantics::Value)
        );
        test!(bytes, "byte[]", TypeRef::new(Type::Bytes, Semantics::Value));

        test!(
            entity_id,
            "a.b.c",
            TypeRef::new(
                Type::Api(EntityId::new_unqualified("a.b.c")),
                Semantics::Value
            )
        );

        // Vec/Array.
        test!(
            array,
            "int[]",
            TypeRef::new(
                Type::new_array(TypeRef::new(Type::I32, Semantics::Value)),
                Semantics::Value
            )
        );
        test!(
            array_nested,
            "int[][][]",
            TypeRef::new(
                Type::new_array(TypeRef::new(
                    Type::new_array(TypeRef::new(
                        Type::new_array(TypeRef::new(Type::I32, Semantics::Value)),
                        Semantics::Value
                    )),
                    Semantics::Value
                )),
                Semantics::Value
            )
        );
        test!(
            complex_array,
            "List<int[]>[]",
            TypeRef::new(
                Type::new_array(TypeRef::new(
                    Type::new_array(TypeRef::new(
                        Type::new_array(TypeRef::new(Type::I32, Semantics::Value)),
                        Semantics::Value
                    )),
                    Semantics::Value
                )),
                Semantics::Value
            )
        );
        test!(
            array_of_optional,
            "int?[]",
            TypeRef::new(
                Type::new_array(TypeRef::new(
                    Type::new_optional(TypeRef::new(Type::I32, Semantics::Value)),
                    Semantics::Value
                )),
                Semantics::Value
            )
        );
        test!(
            vec,
            "List<int>",
            TypeRef::new(
                Type::new_array(TypeRef::new(Type::I32, Semantics::Value)),
                Semantics::Value
            )
        );
        test!(
            vec_api,
            "List<a.b.c>",
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
            "List<List<List<string>>>",
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
            "Dictionary<string, int>",
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
            "Dictionary<dto, a.b.c>",
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
            "Dictionary<string, Dictionary<Dictionary<int, float>, string>>",
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
            "int?",
            TypeRef::new(
                Type::Optional(Box::new(TypeRef::new(Type::I32, Semantics::Value))),
                Semantics::Value
            )
        );
        test!(
            option_api,
            "a.b.c?",
            TypeRef::new(
                Type::new_optional(TypeRef::new(
                    Type::Api(EntityId::new_unqualified("a.b.c")),
                    Semantics::Value
                )),
                Semantics::Value
            )
        );
        test!(
            complex_option,
            "List<int?[]>",
            TypeRef::new(
                Type::new_array(TypeRef::new(
                    Type::new_array(TypeRef::new(
                        Type::new_optional(TypeRef::new(Type::I32, Semantics::Value)),
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
            "Dictionary<List<string>, List<string>>",
            TypeRef::new(
                Type::new_map(
                    TypeRef::new(
                        Type::new_array(TypeRef::new(Type::String, Semantics::Value)),
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

        use crate::parser::ty::user_ty;
        use apyxl::parser::{Config, UserType};

        #[test]
        fn test() {
            let config = Config {
                user_types: vec![
                    UserType {
                        parse: "int".to_string(),
                        name: "i32".to_string(),
                    },
                    UserType {
                        parse: "float".to_string(),
                        name: "f32".to_string(),
                    },
                ],
                ..Default::default()
            };
            let ty = user_ty(&config).parse("int").into_output().unwrap();
            assert_eq!(ty, "i32");
            let ty = user_ty(&config).parse("float").into_output().unwrap();
            assert_eq!(ty, "f32");
        }
    }

    mod entity_id {
        use anyhow::Result;
        use chumsky::Parser;
        use itertools::Itertools;

        use crate::parser::ty::entity_id;
        use apyxl::parser::test_util::wrap_test_err;

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
                .parse("a.b.c")
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
