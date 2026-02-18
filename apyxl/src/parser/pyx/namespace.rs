use crate::model::{Namespace, NamespaceChild};
use crate::parser::pyx::attributes;
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

        attributes::user()
            .then_ignore(just(Token::Namespace))
            .then(ident)
            .then(children.delimited_by(just(Token::Ctrl('{')), just(Token::Ctrl('}'))))
            .map_with(|((user, name), children), e| {
                let mut ns = Namespace {
                    name: Cow::Borrowed(name),
                    children,
                    attributes: Default::default(),
                    is_virtual: false,
                };
                ns.attributes.user = user;
                ns.with_span(e.span())
            })
            .boxed()
    })
}
