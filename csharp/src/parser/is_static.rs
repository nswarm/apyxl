use chumsky::Parser;
use apyxl::parser::error::Error;
use apyxl::parser::util;

pub fn is_static<'a>() -> impl Parser<'a, &'a str, bool, Error<'a>> {
    util::keyword_ex("static")
        .padded()
        .or_not()
        .map(|x| x.is_some())
}
