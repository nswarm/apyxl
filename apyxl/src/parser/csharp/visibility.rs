use crate::parser::error::Error;
use crate::parser::{util, Config};
use chumsky::{text, Parser};
use chumsky::primitive::choice;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Visibility {
    Public,
    Protected,
    Private,
    Internal,
}

impl Visibility {
    pub fn is_visible(&self, config: &Config) -> bool {
        *self == Visibility::Public || (config.enable_parse_private)
    }

    pub fn filter<T>(&self, value: T, config: &Config) -> Option<T> {
        if self.is_visible(config) {
            Some(value)
        } else {
            None
        }
    }
}

pub fn parser<'a>() -> impl Parser<'a, &'a str, Visibility, Error<'a>> {
    choice((
        util::keyword_ex("public"),
        util::keyword_ex("protected"),
        util::keyword_ex("private"),
        util::keyword_ex("internal"),
    ))
        .then_ignore(text::whitespace().at_least(1))
        .map(|s| match s {
            "public" => Visibility::Public,
            "protected" => Visibility::Protected,
            "private" => Visibility::Private,
            "internal" => Visibility::Internal,
            _ => unreachable!(),
        })
        .or_not()
        // None = Private is not technically true, e.g. struct defaults public
        .map(|o| o.unwrap_or_else(|| Visibility::Private))
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use chumsky::Parser;
    use crate::parser::csharp::visibility;

    #[test]
    fn requires_whitespace() -> Result<()> {
        let result = visibility::parser()
            .parse("public")
            .into_result();
        assert!(result.is_err());
        Ok(())
    }
}
