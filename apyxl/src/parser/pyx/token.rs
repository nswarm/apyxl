use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Token<'src> {
    Ctrl(char),
    Ident(&'src str),
    Namespace,
    AttrOpen,
}

impl fmt::Display for Token<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Token::Ctrl(c) => write!(f, "{c}"),
            Token::Ident(s) => write!(f, "{s}"),
            Token::Namespace => write!(f, "namespace"),
            Token::AttrOpen => write!(f, "#["),
        }
    }
}
