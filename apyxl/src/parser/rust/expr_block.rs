use crate::model::Comment;
use crate::parser::comment;
use crate::parser::error::Error;
use chumsky::prelude::*;
use chumsky::Parser;

#[derive(Debug, PartialEq, Eq)]
pub enum ExprBlock<'a> {
    Comment(Comment<'a>),
    Body(&'a str),
    Nested(Vec<ExprBlock<'a>>),
}

pub fn parser<'a>() -> impl Parser<'a, &'a str, Vec<ExprBlock<'a>>, Error<'a>> {
    let body = none_of("{}").repeated().at_least(1).slice().map(&str::trim);
    recursive(|nested| {
        choice((
            comment::comment().boxed().padded().map(ExprBlock::Comment),
            nested.map(ExprBlock::Nested),
            body.map(ExprBlock::Body),
        ))
        .repeated()
        .collect::<Vec<_>>()
        .delimited_by(just('{').padded(), just('}').padded())
        .recover_with(via_parser(nested_delimiters('{', '}', [], |_| vec![])))
    })
}

#[cfg(test)]
mod tests {
    use chumsky::{text, Parser};

    use crate::model::Comment;
    use crate::parser::rust::expr_block;
    use crate::parser::rust::expr_block::ExprBlock;

    #[test]
    fn complex() {
        let result = expr_block::parser()
            .parse("{left{inner1_left{inner1}inner1_right}middle{inner2}{inner3}right}")
            .into_result();
        assert_eq!(
            result.unwrap(),
            vec![
                ExprBlock::Body("left"),
                ExprBlock::Nested(vec![
                    ExprBlock::Body("inner1_left"),
                    ExprBlock::Nested(vec![ExprBlock::Body("inner1"),]),
                    ExprBlock::Body("inner1_right"),
                ]),
                ExprBlock::Body("middle"),
                ExprBlock::Nested(vec![ExprBlock::Body("inner2"),]),
                ExprBlock::Nested(vec![ExprBlock::Body("inner3"),]),
                ExprBlock::Body("right"),
            ]
        );
    }

    #[test]
    fn empty() {
        let result = expr_block::parser().parse("{}").into_result();
        assert_eq!(result.unwrap(), vec![]);
    }

    #[test]
    fn arbitrary_content() {
        let result = expr_block::parser()
            .parse(
                r#"{
            1234 !@#$%^&*()_+-= asdf
        }"#,
            )
            .into_result();
        assert_eq!(
            result.unwrap(),
            vec![ExprBlock::Body("1234 !@#$%^&*()_+-= asdf")]
        );
    }

    #[test]
    fn line_comment() {
        let result = expr_block::parser()
            .parse(
                r#"
                { // don't break! }
                }"#,
            )
            .into_result();
        assert_eq!(
            result.unwrap(),
            vec![ExprBlock::Comment(Comment::unowned(&["don't break! }"]))],
        );
    }

    #[test]
    fn block_comment() {
        let result = expr_block::parser()
            .parse(
                r#"{
                { /* don't break! {{{ */ }
                }"#,
            )
            .into_result();
        assert_eq!(
            result.unwrap(),
            vec![ExprBlock::Nested(vec![ExprBlock::Comment(
                Comment::unowned(&["don't break! {{{"])
            )])]
        );
    }

    #[test]
    fn continues_parsing_after() {
        let result = expr_block::parser()
            .padded()
            .ignore_then(text::ident().padded())
            .parse(
                r#"
            {
              ignored stuff
            }
            not_ignored
            "#,
            )
            .into_result();
        assert!(result.is_ok(), "parse should not fail");
        assert_eq!(result.unwrap(), "not_ignored");
    }
}
