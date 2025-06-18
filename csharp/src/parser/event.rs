// use crate::parser::is_static::is_static;
// use crate::parser::visibility::Visibility;
// use crate::parser::{attributes, comment, ty, visibility};
// use apyxl::model::{Attributes, Field};
// use apyxl::parser::error::Error;
// use apyxl::parser::{util, Config};
// use chumsky::prelude::{any, just};
// use chumsky::{text, Parser};
//
// pub fn parser(config: &Config) -> impl Parser<&str, (Field, Visibility), Error> {
//     let end = just(';');
//     let initializer = just('=')
//         .padded()
//         .then(any().and_is(end.not()).repeated().slice());
//     let field = ty::parser(config)
//         .then_ignore(text::whitespace().at_least(1))
//         .then(text::ident())
//         .then_ignore(initializer.or_not())
//         .then_ignore(end.padded());
//     // todo properties
//     //      x => thing;
//     //      x => { thing }
//     //      x { get/set => thing; }
//     //      x { get/set { thing } }
//     //      x { get/set { thing } } = thing;
//     //      x { get; set; }
//     //      x { get; private set; }
//     // todo events
//     comment::multi()
//         .then(attributes::attributes().padded())
//         .then(visibility::parser(Visibility::Private))
//         .then(is_static())
//         .then_ignore(util::keyword_ex("readonly").or_not())
//         .then(field)
//         .map(
//             |((((comments, user), visibility), is_static), (ty, name))| {
//                 (
//                     Field {
//                         name,
//                         ty,
//                         attributes: Attributes {
//                             comments,
//                             user,
//                             ..Default::default()
//                         },
//                         is_static,
//                     },
//                     visibility,
//                 )
//             },
//         )
// }
