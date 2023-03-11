use anyhow::Result;

use nom::branch::alt;
use nom::bytes::complete::{tag, take_until, take_while1};
use nom::character::complete::{line_ending, not_line_ending};
use nom::multi::many0;
use nom::sequence::terminated;
use nom::IResult;

use crate::input::Input;
use crate::model::{Dto, Model};
use crate::parser::Parser;

/// Parser that breaks up the input by the given delimiter.
/// Newlines (either \r\n or just \n) also count as a delimiter.
pub struct Delimited {
    delimiter: String,
}

impl Delimited {
    pub fn new(delimiter: impl ToString) -> Self {
        Self {
            delimiter: delimiter.to_string(),
        }
    }
}

impl Parser for Delimited {
    fn parse(&self, input: &dyn Input) -> Result<Model> {
        let tokens = finished_tokens(input.data(), &self.delimiter)?;
        let model = Model {
            dtos: tokens
                .into_iter()
                .map(|t| Dto { name: t.to_owned() })
                .collect::<Vec<Dto>>(),
        };
        Ok(model)
    }
}

fn tokens<'a>(input: &'a str, delimiter: &'a str) -> IResult<&'a str, Vec<&'a str>> {
    let token_delimited = terminated(take_until(delimiter), tag(delimiter));
    let token_eol = terminated(not_line_ending, line_ending);
    let token_rest = take_while1(|_| true);
    let token = alt((token_delimited, token_eol, token_rest));
    let mut tokens = many0(token);
    tokens(input)
}

fn finished_tokens<'a>(input: &'a str, delimiter: &'a str) -> Result<Vec<&'a str>> {
    let (_, res) = tokens(input, delimiter).map_err(|err| err.to_owned())?;
    Ok(res)
}

#[cfg(test)]
mod test {
    use anyhow::Result;

    use crate::delimited::finished_tokens;

    #[test]
    fn tokens_single_char() -> Result<()> {
        assert_eq!(
            finished_tokens("abc,def,ghi", ",")?,
            vec!["abc", "def", "ghi"]
        );
        Ok(())
    }

    #[test]
    fn tokens_multi_char() -> Result<()> {
        assert_eq!(
            finished_tokens("abc:::def:::ghi", ":::")?,
            vec!["abc", "def", "ghi"]
        );
        Ok(())
    }

    #[test]
    fn tokens_eol_rc_nl() -> Result<()> {
        assert_eq!(
            finished_tokens("abc,def\r\nghi", ",")?,
            vec!["abc", "def", "ghi"]
        );
        Ok(())
    }

    #[test]
    fn tokens_eol_nl() -> Result<()> {
        assert_eq!(
            finished_tokens("abc,def\nghi", ",")?,
            vec!["abc", "def", "ghi"]
        );
        Ok(())
    }
}
