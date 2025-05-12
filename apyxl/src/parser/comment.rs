use std::borrow::Cow;

use chumsky::prelude::{any, choice, just};
use chumsky::{text, IterParser, Parser};

use crate::model::Comment;
use crate::parser::error::Error;

/// Parses a block comment starting with `start` and ending with `end`. The entire contents will be
/// a single element in the vec. This also does not currently handle indentation very well, so the
/// indentation from the source will be present in the comment data.
///
/// With start=just("/*"), end=just("*/")
/// ```
/// /*
/// i am
///     a multiline
/// comment
/// */
/// ```
/// would result in
/// `vec!["i am\n    a multiline\ncomment"]`
pub fn block_comment<'a>(
    start: impl Parser<'a, &'a str, &'a str, Error<'a>>,
    end: impl Parser<'a, &'a str, &'a str, Error<'a>> + Clone,
) -> impl Parser<'a, &'a str, Comment<'a>, Error<'a>> {
    any()
        .and_is(end.clone().not())
        .repeated()
        .slice()
        .map(&str::trim)
        .delimited_by(start, end)
        .map(|s| {
            if !s.is_empty() {
                Comment::from(vec![s])
            } else {
                Comment::default()
            }
        })
}

/// Parses a line comment where each line starts with `start`. Each line is an element in the returned
/// vec without the prefixed `start`, including all padding and empty lines.
///
/// With start=just("//")
/// ```
/// // i am
/// //     a multiline
/// // comment
/// //
/// ```
/// would result in
/// `vec!["i am", "    a multiline", "comment", ""]`
pub fn line_comment<'a>(
    start: impl Parser<'a, &'a str, &'a str, Error<'a>>,
) -> impl Parser<'a, &'a str, Comment<'a>, Error<'a>> {
    let text = any().and_is(just('\n').not()).repeated().slice();
    let line_start = start.then(just(' ').or_not());
    let line = text::inline_whitespace()
        .then(line_start)
        .ignore_then(text)
        .then_ignore(just('\n'));
    line.map(Cow::Borrowed)
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .map(|v| v.into())
}

/// Parses a single line or block comment group. Each line is an element in the returned vec.
pub fn single<'a>(
    line: impl Parser<'a, &'a str, &'a str, Error<'a>>,
    block_start: impl Parser<'a, &'a str, &'a str, Error<'a>>,
    block_end: impl Parser<'a, &'a str, &'a str, Error<'a>> + Clone,
) -> impl Parser<'a, &'a str, Comment<'a>, Error<'a>> {
    choice((line_comment(line), block_comment(block_start, block_end)))
}

/// Parses zero or more [single]s into a Vec.
pub fn multi<'a>(
    line: impl Parser<'a, &'a str, &'a str, Error<'a>>,
    block_start: impl Parser<'a, &'a str, &'a str, Error<'a>>,
    block_end: impl Parser<'a, &'a str, &'a str, Error<'a>> + Clone,
) -> impl Parser<'a, &'a str, Vec<Comment<'a>>, Error<'a>> {
    single(line, block_start, block_end)
        .padded()
        .repeated()
        .collect::<Vec<_>>()
}

#[cfg(test)]
mod tests {
    use crate::model::Comment;
    use crate::parser::comment;
    use crate::parser::error::Error;
    use crate::parser::test_util::wrap_test_err;
    use anyhow::Result;
    use chumsky::prelude::{choice, just};
    use chumsky::Parser;

    #[test]
    fn empty_comment_err() {
        assert!(single().parse("").into_result().is_err());
    }

    #[test]
    fn line_comment() -> Result<()> {
        let value = single()
            .parse("// line comment\n")
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(value, Comment::unowned(&["line comment"]));
        Ok(())
    }

    #[test]
    fn line_comment_alt() -> Result<()> {
        let value = single()
            .parse("/// line comment\n")
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(value, Comment::unowned(&["line comment"]));
        Ok(())
    }

    #[test]
    fn line_comment_multi_with_spacing() -> Result<()> {
        let value = single()
            .parse(
                r#"//
                // line one
                //     line two
                // line three
                //
"#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(
            value,
            Comment::unowned(&["", "line one", "    line two", "line three", ""])
        );
        Ok(())
    }

    #[test]
    fn block_comment() -> Result<()> {
        let value = single()
            .parse("/* block comment */")
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(value, Comment::unowned(&["block comment"]));
        Ok(())
    }

    #[test]
    fn test_multi_comment() -> Result<()> {
        let value = multi()
            .parse(
                r#"
                    /* line one */
                    // line two
                    // line three

                    /// line four
                    /** line five */
                    /* line six */
                "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(
            value,
            vec![
                Comment::unowned(&["line one"]),
                Comment::unowned(&["line two", "line three"]),
                Comment::unowned(&["line four"]),
                Comment::unowned(&["line five"]),
                Comment::unowned(&["line six"]),
            ]
        );
        Ok(())
    }

    fn line<'a>() -> impl Parser<'a, &'a str, &'a str, Error<'a>> {
        choice((just("///"), just("//")))
    }

    fn begin<'a>() -> impl Parser<'a, &'a str, &'a str, Error<'a>> {
        choice((just("/**"), just("/*")))
    }

    fn single<'a>() -> impl Parser<'a, &'a str, Comment<'a>, Error<'a>> {
        comment::single(line(), begin(), just("*/"))
    }

    fn multi<'a>() -> impl Parser<'a, &'a str, Vec<Comment<'a>>, Error<'a>> {
        comment::multi(line(), begin(), just("*/"))
    }
}
