use crate::model::attributes;
use crate::model::Comment;
use crate::parser::pyx::token::Token;
use crate::parser::pyx::Error;
use chumsky::input::ValueInput;
use chumsky::prelude::*;
use std::borrow::Cow;

/// Parses zero or more comment tokens into `Vec<Comment>`.
/// Each token becomes one `Comment`. Separate tokens (separated by blank lines
/// for line comments, or distinct `/* */` blocks) become separate entries.
pub fn comments<'tok, 'src: 'tok, I>(
) -> impl Parser<'tok, I, Vec<Comment<'src>>, Error<'tok, 'src>>
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan>,
{
    select! { Token::Comment(lines) => Comment::from(lines) }
    .repeated()
    .collect::<Vec<_>>()
}

/// Parses a single data entry: `value` or `key=value`.
fn data_entry<'tok, 'src: 'tok, I>(
) -> impl Parser<'tok, I, attributes::UserData<'src>, Error<'tok, 'src>>
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan>,
{
    let ident = select! { Token::Ident(ident) => ident };
    ident
        .then(just(Token::Ctrl('=')).ignore_then(ident).or_not())
        .map(|(lhs, rhs)| match rhs {
            None => attributes::UserData::new(None, lhs),
            Some(rhs) => attributes::UserData::new(Some(lhs), rhs),
        })
}

/// Parses a parenthesized data list: `(a, b, k=v)`.
fn data_list<'tok, 'src: 'tok, I>(
) -> impl Parser<'tok, I, Vec<attributes::UserData<'src>>, Error<'tok, 'src>>
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan>,
{
    data_entry()
        .separated_by(just(Token::Ctrl(',')))
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(just(Token::Ctrl('(')), just(Token::Ctrl(')')))
}

/// Parses a single user attribute: `flag`, `name(a, b)`, or `name(k=v)`.
fn single<'tok, 'src: 'tok, I>(
) -> impl Parser<'tok, I, attributes::User<'src>, Error<'tok, 'src>>
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan>,
{
    let ident = select! { Token::Ident(ident) => ident };
    ident.then(data_list().or_not()).map(|(name, data)| attributes::User {
        name: Cow::Borrowed(name),
        data: data.unwrap_or_default(),
    })
}

/// Parses `#[attr1, attr2(a, b), attr3(k=v)]` into `Vec<User>`.
/// Returns empty vec if no attribute block is present.
pub fn user<'tok, 'src: 'tok, I>(
) -> impl Parser<'tok, I, Vec<attributes::User<'src>>, Error<'tok, 'src>>
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan>,
{
    single()
        .separated_by(just(Token::Ctrl(',')))
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(just(Token::AttrOpen), just(Token::Ctrl(']')))
        .or_not()
        .map(|opt| opt.unwrap_or_default())
}
