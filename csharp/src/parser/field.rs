use crate::parser::is_static::is_static;
use crate::parser::visibility::Visibility;
use crate::parser::{attributes, comment, ty, visibility};
use apyxl::model::{Attributes, Field};
use apyxl::parser::error::Error;
use apyxl::parser::{util, Config};
use chumsky::prelude::{any, just};
use chumsky::{text, Parser};

pub fn parser(config: &Config) -> impl Parser<&str, (Field, Visibility), Error> {
    let end = just(';');
    let initializer = just('=')
        .padded()
        .then(any().and_is(end.not()).repeated().slice());
    let field = ty::parser(config)
        .then_ignore(text::whitespace().at_least(1))
        .then(text::ident())
        .then_ignore(initializer.or_not())
        .then_ignore(end.padded());
    comment::multi()
        .then(attributes::attributes().padded())
        .then(visibility::parser(Visibility::Private))
        .then(is_static())
        .then_ignore(util::keyword_ex("const").padded().or_not())
        .then_ignore(util::keyword_ex("readonly").padded().or_not())
        .then_ignore(util::keyword_ex("event").padded().or_not())
        .then(field)
        .map(
            |((((comments, user), visibility), is_static), (ty, name))| {
                (
                    Field {
                        name,
                        ty,
                        attributes: Attributes {
                            comments,
                            user,
                            ..Default::default()
                        },
                        is_static,
                    },
                    visibility,
                )
            },
        )
}

// more tests in dto.

#[cfg(test)]
mod tests {
    use crate::parser::field::parser;
    use anyhow::Result;
    use apyxl::parser::test_util::wrap_test_err;
    use apyxl::test_util::executor::TEST_CONFIG;
    use chumsky::Parser;

    #[test]
    fn const_field() -> Result<()> {
        let input = "public const int field = 0;";
        let _ = parser(&TEST_CONFIG)
            .parse(input)
            .into_result()
            .map_err(wrap_test_err)?;
        Ok(())
    }

    #[test]
    fn complex_field() -> Result<()> {
        let input = "public static Dictionary<string, List<int>> field = 0;";
        let _ = parser(&TEST_CONFIG)
            .parse(input)
            .into_result()
            .map_err(wrap_test_err)?;
        Ok(())
    }
}
