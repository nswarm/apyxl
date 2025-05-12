use crate::model::Comment;
use crate::parser::comment;
use crate::parser::error::Error;
use chumsky::prelude::just;
use chumsky::Parser;

pub fn single<'a>() -> impl Parser<'a, &'a str, Comment<'a>, Error<'a>> {
    comment::single(line_start(), block_start(), block_end())
}

pub fn multi<'a>() -> impl Parser<'a, &'a str, Vec<Comment<'a>>, Error<'a>> {
    comment::multi(line_start(), block_start(), block_end())
}

fn line_start<'a>() -> impl Parser<'a, &'a str, &'a str, Error<'a>> {
    just("//")
}

fn block_start<'a>() -> impl Parser<'a, &'a str, &'a str, Error<'a>> {
    just("/*")
}

fn block_end<'a>() -> impl Parser<'a, &'a str, &'a str, Error<'a>> + Clone {
    just("*/")
}
