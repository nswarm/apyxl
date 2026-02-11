use crate::model::Namespace;
use crate::parser::pyx::token::Token;
use crate::parser::pyx::Error;
use crate::parser::Config;
use chumsky::input::ValueInput;
use chumsky::prelude::*;
use std::borrow::Cow;

pub fn parser<'tok, 'src: 'tok, I>(
    config: &Config,
) -> impl Parser<'tok, I, Spanned<Namespace<'src>>, Error<'tok, 'src>>
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan>,
{
    let ident = select! { Token::Ident(ident) => ident };

    just(Token::Namespace)
        .ignore_then(ident)
        .then_ignore(just(Token::Ctrl('{')))
        .then_ignore(just(Token::Ctrl('}')))
        .map_with(|name, e| {
            Namespace {
                name: Cow::Borrowed(name),
                children: vec![],
                attributes: Default::default(),
                is_virtual: false,
            }
            .with_span(e.span())
        })
}

// fn children<'src, I>(
//     config: &'src Config,
//     // namespace: impl Parser<'src, &'src str, Namespace<'src>, Error<'src>>,
//     // end_delimiter: impl Parser<'src, &'src str, (), Error<'src>>,
// ) -> impl Parser<'src, &'src str, Vec<NamespaceChild<'src>>, Error<'src>>
// where
//     I: ValueInput<'src, Token = Token<'src>, Span = SimpleSpan>,
// {
//
//     // choice((
//     //     crate::parser::rust::dto::parser(config).map(|(c, v)| Some((NamespaceChild::Dto(c), v))),
//     //     crate::parser::rust::rpc::parser(config).map(|(c, v)| Some((NamespaceChild::Rpc(c), v))),
//     //     crate::parser::rust::en::parser().map(|(c, v)| Some((NamespaceChild::Enum(c), v))),
//     //     crate::parser::rust::ty_alias::parser(config).map(|(c, v)| Some((NamespaceChild::TypeAlias(c), v))),
//     //     crate::parser::rust::namespace::field(config).map(|(c, v)| Some((NamespaceChild::Field(c), v))),
//     //     namespace.map(|(c, v)| Some((NamespaceChild::Namespace(c), v))),
//     //     crate::parser::rust::namespace::impl_block(config).map(|c| Some((NamespaceChild::Namespace(c), crate::parser::rust::visibility::Visibility::Public))),
//     // ))
//     //     .recover_with(skip_then_retry_until(
//     //         any().ignored(),
//     //         end_delimiter.ignored(),
//     //     ))
//     //     .map(|opt| match opt {
//     //         Some((child, visibility)) => visibility.filter(child, config),
//     //         None => None,
//     //     })
//     //     .repeated()
//     //     .collect::<Vec<_>>()
//     //     .map(|v| v.into_iter().flatten().collect_vec())
//     //     .then_ignore(crate::parser::rust::comment::multi())
// }
