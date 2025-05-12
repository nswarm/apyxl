use crate::parser::error::Error;
use chumsky::error::Rich;
use chumsky::{Parser, text};

/// Expanded [text::keyword] that has a more informative error.
pub fn keyword_ex(keyword: &str) -> impl Parser<&str, &str, Error> {
    text::ident()
        .try_map(move |s: &str, span| {
            if s == keyword {
                Ok(())
            } else {
                Err(Rich::custom(span, format!("found unexpected token {}", s)))
            }
        })
        .slice()
}
