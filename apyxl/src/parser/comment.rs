use std::borrow::Cow;

use chumsky::prelude::{any, choice, just};
use chumsky::{text, IterParser, Parser};

use crate::model::Comment;
use crate::parser::error::Error;

/// Parses a block comment starting with `/*` and ending with `*/`. The entire contents will be
/// a single element in the vec. This also does not currently handle indentation very well, so the
/// indentation from the source will be present in the comment data.
///
/// ```
/// /*
/// i am
///     a multiline
/// comment
/// */
/// ```
/// would result in
/// `vec!["i am\n    a multiline\ncomment"]`
pub fn block_comment<'a>() -> impl Parser<'a, &'a str, Comment<'a>, Error<'a>> {
    any()
        .and_is(just("*/").not())
        .repeated()
        .slice()
        .map(&str::trim)
        .delimited_by(just("/*"), just("*/"))
        .map(|s| {
            if !s.is_empty() {
                Comment::from(vec![s])
            } else {
                Comment::default()
            }
        })
}

/// Parses a line comment where each line starts with `//`. Each line is an element in the returned
/// vec without the prefixed `//`, including all padding and empty lines.
///
/// ```
/// // i am
/// //     a multiline
/// // comment
/// //
/// ```
/// would result in
/// `vec!["i am", "    a multiline", "comment", ""]`
pub fn line_comment<'a>() -> impl Parser<'a, &'a str, Comment<'a>, Error<'a>> {
    let text = any().and_is(just('\n').not()).repeated().slice();
    let line_start = just("//").then(just(' ').or_not());
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
pub fn comment<'a>() -> impl Parser<'a, &'a str, Comment<'a>, Error<'a>> {
    choice((line_comment(), block_comment()))
}

/// Parses zero or more [comment]s into a Vec.
pub fn multi_comment<'a>() -> impl Parser<'a, &'a str, Vec<Comment<'a>>, Error<'a>> {
    comment().padded().repeated().collect::<Vec<_>>()
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use chumsky::Parser;

    use crate::model::Comment;
    use crate::parser::comment::{comment, multi_comment};
    use crate::parser::test_util::wrap_test_err;

    #[test]
    fn empty_comment_err() {
        assert!(comment().parse("").into_result().is_err());
    }

    #[test]
    fn line_comment() -> Result<()> {
        let value = comment()
            .parse("// line comment\n")
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(value, Comment::unowned(&["line comment"]));
        Ok(())
    }

    #[test]
    fn line_comment_multi_with_spacing() -> Result<()> {
        let value = comment()
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
        let value = comment()
            .parse("/* block comment */")
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(value, Comment::unowned(&["block comment"]));
        Ok(())
    }

    #[test]
    fn test_multi_comment() -> Result<()> {
        let value = multi_comment()
            .parse(
                r#"
                    /* line one */
                    // line two
                    // line three

                    // line four
                    /* line five */
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
}
