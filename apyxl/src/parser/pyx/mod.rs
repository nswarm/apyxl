mod error;
mod lexer;
mod namespace;
mod token;

use crate::model::Api;
use crate::parser::pyx::lexer::lexer;
use crate::parser::pyx::token::Token;
use crate::parser::Config;
use crate::{model, Input, Parser};
use anyhow::Result;
use chumsky::input::ValueInput;
use chumsky::prelude::Input as ChumskyInput;
use chumsky::prelude::Parser as ChumskyParser;
use chumsky::prelude::*;

type Error<'tok, 'src> = error::Error<'tok, Token<'src>>;

#[derive(Default)]
pub struct Pyx {}

impl Parser for Pyx {
    fn parse<'a, I: Input + 'a>(
        &self,
        config: &'a Config,
        input: &'a mut I,
        builder: &mut model::Builder<'a>,
    ) -> Result<()> {
        for (chunk, data) in input.chunks() {
            let mut errs = Vec::new();
            let tokens = lex(data, &mut errs);
            let api = parse(config, data.len(), &tokens, &mut errs);

            // todo some sort of Input span, file id, whatever...

            if let Some(api) = api {
                builder.merge_from_chunk(api, chunk);
            }

            error::report(chunk, data, errs);
        }

        Ok(())
    }
}

fn lex<'tok, 'src: 'tok>(
    data: &'src str,
    errs: &mut Vec<Rich<'tok, String>>,
) -> Option<Vec<Spanned<Token<'src>>>> {
    let (tokens, token_errs) = lexer().parse(data).into_output_errors();

    errs.extend(
        token_errs
            .into_iter()
            .map(|err| err.map_token(|t| t.to_string())),
    );

    tokens
}

fn parse<'tok, 'src: 'tok>(
    config: &Config,
    src_len: usize,
    tokens: &'tok Option<Vec<Spanned<Token<'src>>>>,
    errs: &mut Vec<Rich<'tok, String>>,
) -> Option<Api<'src>> {
    let tokens = match &tokens {
        None => return None,
        Some(tokens) => tokens,
    };

    let (api, parse_errs) = api_parser(config)
        .parse(tokens.as_slice().split_spanned((src_len..src_len).into()))
        .into_output_errors();

    errs.extend(
        parse_errs
            .into_iter()
            .map(|err| err.map_token(|t| t.to_string())),
    );

    api
}

pub fn api_parser<'tok, 'src: 'tok, I>(
    config: &Config,
) -> impl chumsky::Parser<'tok, I, Api<'src>, Error<'tok, 'src>>
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan>,
{
    // Don't need the span anymore.
    namespace::parser(config).map(|ns| ns.inner)
}
