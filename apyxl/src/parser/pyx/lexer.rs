use crate::parser::pyx::token::Token;
use chumsky::prelude::*;

pub fn lexer<'src>(
) -> impl Parser<'src, &'src str, Vec<Spanned<Token<'src>>>, extra::Err<Rich<'src, char, SimpleSpan>>>
{
    let ctrl = one_of("{}").map(Token::Ctrl);

    let ident = text::ascii::ident().map(|ident: &str| match ident {
        "namespace" => Token::Namespace,
        _ => Token::Ident(ident),
    });

    let token = choice((ctrl, ident));

    token
        .map_with(|tok, e| Spanned {
            inner: tok,
            span: e.span(),
        })
        // .padded_by(comment.repeated())
        .padded()
        // If we encounter an error, skip and attempt to lex the next character as a token instead
        .recover_with(skip_then_retry_until(any().ignored(), end()))
        .repeated()
        .collect()
}
