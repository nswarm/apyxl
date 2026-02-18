use crate::model::{Namespace, NamespaceChild};
use crate::parser::pyx::token::Token;
use crate::parser::pyx::Error;
use crate::parser::Config;
use chumsky::input::ValueInput;
use chumsky::prelude::*;
use std::borrow::Cow;

pub fn parser<'tok, 'src: 'tok, I>(
    config: &Config,
) -> impl Parser<'tok, I, Spanned<Namespace<'src>>, Error<'tok, 'src>>
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan>,
{
    let ident = select! { Token::Ident(ident) => ident };

    recursive(|nested| {
        let children = nested
            .map(|ns: Spanned<Namespace<'src>>| NamespaceChild::Namespace(ns.inner))
            .repeated()
            .collect::<Vec<_>>();

        just(Token::Namespace)
            .ignore_then(ident)
            .then(children.delimited_by(just(Token::Ctrl('{')), just(Token::Ctrl('}'))))
            .map_with(|(name, children), e| {
                Namespace {
                    name: Cow::Borrowed(name),
                    children,
                    attributes: Default::default(),
                    is_virtual: false,
                }
                .with_span(e.span())
            })
            .boxed()
    })
}
