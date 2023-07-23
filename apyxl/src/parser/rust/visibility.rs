use crate::model::NamespaceChild;
use crate::parser::error::Error;
use crate::parser::{util, Config};
use chumsky::{text, Parser};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Visibility {
    Public,
    Private,
}

impl Visibility {
    pub fn is_visible(&self, config: &Config) -> bool {
        *self == Visibility::Public || (*self == Visibility::Private && config.enable_parse_private)
    }

    pub fn filter_child<'a>(
        &self,
        child: NamespaceChild<'a>,
        config: &Config,
    ) -> Option<NamespaceChild<'a>> {
        if self.is_visible(config) {
            Some(child)
        } else {
            None
        }
    }
}

pub fn parser<'a>() -> impl Parser<'a, &'a str, Visibility, Error<'a>> {
    util::keyword_ex("pub")
        .then(text::whitespace().at_least(1))
        .or_not()
        .map(|o| match o {
            None => Visibility::Private,
            Some(_) => Visibility::Public,
        })
}
