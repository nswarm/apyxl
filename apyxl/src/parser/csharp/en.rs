use chumsky::prelude::*;

use crate::model::{Attributes, Enum, EnumValue, EnumValueNumber};
use crate::parser::error::Error;
use crate::parser::csharp::visibility::Visibility;
use crate::parser::csharp::{attributes, comment, visibility};
use crate::parser::util;

const INVALID_ENUM_NUMBER: EnumValueNumber = EnumValueNumber::MAX;

pub fn parser<'a>() -> impl Parser<'a, &'a str, (Enum<'a>, Visibility), Error<'a>> {
    let prefix = util::keyword_ex("enum").then(text::whitespace().at_least(1));
    let name = text::ident();
    let values = en_value()
        .separated_by(just(',').padded())
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(just('{').padded(), just('}').padded());
    comment::multi()
        .then(attributes::attributes().padded())
        .then(visibility::parser())
        .then_ignore(prefix)
        .then(name)
        .then(values)
        .map(|((((comments, user), visibility), name), values)| {
            (
                Enum {
                    name,
                    values: apply_enum_value_number_defaults(values),
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

fn en_value<'a>() -> impl Parser<'a, &'a str, EnumValue<'a>, Error<'a>> {
    let number = just('=')
        .padded()
        .ignore_then(text::int(10).try_map(|s, span| {
            str::parse::<EnumValueNumber>(s)
                .map_err(|_| chumsky::error::Error::<&'a str>::expected_found(None, None, span))
        }));
    comment::multi()
        .then(attributes::attributes().padded())
        .then(text::ident())
        .then(number.or_not())
        .padded()
        .map(|(((comments, user), name), number)| EnumValue {
            name,
            number: number.unwrap_or(INVALID_ENUM_NUMBER),
            attributes: Attributes {
                comments,
                user,
                ..Default::default()
            },
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

#[cfg(test)]
mod tests {

    mod en_value {
        use anyhow::Result;
        use chumsky::Parser;

        use crate::model::attributes;
        use crate::parser::csharp::en::en_value;
        use crate::parser::test_util::wrap_test_err;

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

        #[test]
        fn attributes() -> Result<()> {
            let value = en_value()
                .parse(
                    r#"
                    [flag1, flag2]
                    Value = 1
                    "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(
                value.attributes.user,
                vec![
                    attributes::User::new_flag("flag1"),
                    attributes::User::new_flag("flag2"),
                ]
            );
            Ok(())
        }
    }

    mod en {
        use anyhow::Result;
        use chumsky::Parser;

        use crate::model::{attributes, Comment, EnumValue, EnumValueNumber};
        use crate::parser::csharp::en;
        use crate::parser::csharp::visibility::Visibility;
        use crate::parser::test_util::wrap_test_err;

        #[test]
        fn public() -> Result<()> {
            let (en, visibility) = en::parser()
                .parse(
                    r#"
                    public enum en {}
                "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(en.name, "en");
            assert_eq!(visibility, Visibility::Public);
            Ok(())
        }

        #[test]
        fn private() -> Result<()> {
            let (en, visibility) = en::parser()
                .parse(
                    r#"
                    enum en {}
                "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(en.name, "en");
            assert_eq!(visibility, Visibility::Private);
            Ok(())
        }

        #[test]
        fn without_numbers() -> Result<()> {
            let (en, _) = en::parser()
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
            let (en, _) = en::parser()
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
            let (en, _) = en::parser()
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

        #[test]
        fn comment() -> Result<()> {
            let (en, _) = en::parser()
                .parse(
                    r#"
            // multi
            // line
            // comment
            enum en {}
            "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(
                en.attributes.comments,
                vec![Comment::unowned(&["multi", "line", "comment"])]
            );
            Ok(())
        }

        #[test]
        fn enum_value_comments() -> Result<()> {
            let (en, _) = en::parser()
                .parse(
                    r#"
                    enum en {
                        // multi
                        // line
                        Value0, /* comment */ Value1,
                        // multi
                        // line
                        // comment
                        Value2,
                    }
                "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(en.values.len(), 3);
            assert_eq!(
                en.values[0].attributes.comments,
                vec![Comment::unowned(&["multi", "line"])]
            );
            assert_eq!(
                en.values[1].attributes.comments,
                vec![Comment::unowned(&["comment"])]
            );
            assert_eq!(
                en.values[2].attributes.comments,
                vec![Comment::unowned(&["multi", "line", "comment"])]
            );
            Ok(())
        }

        #[test]
        fn attributes() -> Result<()> {
            let (en, _) = en::parser()
                .parse(
                    r#"
                    [flag1, flag2]
                    enum Enum {}
                    "#,
                )
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(
                en.attributes.user,
                vec![
                    attributes::User::new_flag("flag1"),
                    attributes::User::new_flag("flag2"),
                ]
            );
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
}
