use crate::parser::pyx::token::Token;
use chumsky::prelude::*;

/// Splits block comment content into lines and strips common leading whitespace.
/// First line (on the `/*` line) is left-trimmed. Continuation lines are dedented
/// by their minimum shared indentation. Empty leading/trailing lines are removed.
fn dedent_block_comment<'a>(content: &'a str) -> Vec<&'a str> {
    let mut lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return lines;
    }

    // First line sits right after /*, just trim leading whitespace.
    lines[0] = lines[0].trim_start();

    // Last line may have trailing whitespace before */, trim it.
    if let Some(last) = lines.last_mut() {
        *last = last.trim_end();
    }

    // Find minimum indentation among non-empty continuation lines.
    let min_indent = lines
        .iter()
        .skip(1)
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);

    // Strip min_indent from continuation lines.
    for line in lines[1..].iter_mut() {
        let strip = min_indent.min(line.len());
        *line = &line[strip..];
    }

    // Remove empty leading/trailing lines.
    while lines.first().is_some_and(|l| l.is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }

    lines
}

pub fn lexer<'src>(
) -> impl Parser<'src, &'src str, Vec<Spanned<Token<'src>>>, extra::Err<Rich<'src, char, SimpleSpan>>>
{
    let line_comment_line = just("///")
        .ignore_then(just(' ').or_not())
        .ignore_then(none_of("\r\n").repeated().to_slice())
        .then_ignore(text::newline())
        .then_ignore(text::inline_whitespace());
    let line_comment = line_comment_line
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .map(Token::Comment);

    let block_comment = any()
        .and_is(just("*/").not())
        .repeated()
        .to_slice()
        .delimited_by(just("/*"), just("*/"))
        .map(dedent_block_comment)
        .map(Token::Comment);

    let attr_open = just("#[").to(Token::AttrOpen);

    let ctrl = one_of("{}](),=").map(Token::Ctrl);

    let ident = text::ascii::ident().map(|ident: &str| match ident {
        "namespace" => Token::Namespace,
        _ => Token::Ident(ident),
    });

    let token = choice((line_comment, block_comment, attr_open, ctrl, ident));

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

#[cfg(test)]
mod tests {
    use super::dedent_block_comment;

    // --- empty / whitespace ---

    /// /**/
    #[test]
    fn empty() {
        assert_eq!(dedent_block_comment(""), Vec::<&str>::new());
    }

    /// /*   */
    #[test]
    fn whitespace_only() {
        assert_eq!(dedent_block_comment("   "), Vec::<&str>::new());
    }

    // --- single line, all four marker positions ---

    /// /* content */
    #[test]
    fn single_line_inline_start_inline_end() {
        assert_eq!(dedent_block_comment(" content "), vec!["content"]);
    }

    /// /* content
    /// */
    #[test]
    fn single_line_inline_start_own_line_end() {
        assert_eq!(dedent_block_comment(" content\n"), vec!["content"]);
    }

    /// /*
    /// content */
    #[test]
    fn single_line_own_line_start_inline_end() {
        assert_eq!(dedent_block_comment("\ncontent "), vec!["content"]);
    }

    /// /*
    /// content
    /// */
    #[test]
    fn single_line_own_line_start_own_line_end() {
        assert_eq!(dedent_block_comment("\ncontent\n"), vec!["content"]);
    }

    // --- multi-line, all four marker positions ---

    /// /* first
    ///    second */
    #[test]
    fn multiline_inline_start_inline_end() {
        assert_eq!(
            dedent_block_comment(" first\n   second "),
            vec!["first", "second"]
        );
    }

    /// /* first
    ///    second
    /// */
    #[test]
    fn multiline_inline_start_own_line_end() {
        assert_eq!(
            dedent_block_comment(" first\n   second\n"),
            vec!["first", "second"]
        );
    }

    /// /*
    ///    first
    ///    second */
    #[test]
    fn multiline_own_line_start_inline_end() {
        assert_eq!(
            dedent_block_comment("\n   first\n   second "),
            vec!["first", "second"]
        );
    }

    /// /*
    ///    first
    ///    second
    /// */
    #[test]
    fn multiline_own_line_start_own_line_end() {
        assert_eq!(
            dedent_block_comment("\n   first\n   second\n"),
            vec!["first", "second"]
        );
    }

    // --- indentation preservation ---

    /// /* a
    ///    multi
    ///       block
    ///    comment */
    #[test]
    fn multiline_preserves_relative_indent() {
        assert_eq!(
            dedent_block_comment(" a\n   multi\n      block\n   comment "),
            vec!["a", "multi", "   block", "comment"]
        );
    }

    // --- blank line preservation ---

    /// /* first
    ///
    ///    last */
    #[test]
    fn blank_line_in_middle_preserved() {
        assert_eq!(
            dedent_block_comment(" first\n\n   last "),
            vec!["first", "", "last"]
        );
    }

    // --- edge cases ---

    /// A continuation line at column 0 anchors min_indent to 0, so other
    /// continuation lines keep their full indentation.
    ///     /*        first
    /// second
    ///       third
    ///     */
    #[test]
    fn continuation_line_outdented_past_marker() {
        assert_eq!(
            dedent_block_comment("        first\nsecond\n      third\n"),
            vec!["first", "second", "      third"]
        );
    }
}
