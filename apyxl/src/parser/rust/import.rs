use crate::model::EntityId;
use crate::parser::error::Error;
use crate::parser::util;
use chumsky::prelude::*;
use itertools::Itertools;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Import {
    /// Single import e.g. `crate::a;` or  `crate::a::Dto;`
    Single(EntityId),

    /// Multi import e.g. `crate::a::{Dto1, Dto2};`
    Multi(Vec<EntityId>),

    /// Blanket module import `e.g. crate::a::*;`
    Blanket(EntityId),
}

pub fn parser<'a>() -> impl Parser<'a, &'a str, Import, Error<'a>> {
    let path = util::keyword_ex("pub")
        .then(text::whitespace().at_least(1))
        .or_not()
        .then(util::keyword_ex("use"))
        .ignore_then(text::whitespace().at_least(1))
        .ignore_then(just("crate::").or_not()) // equivalent to root ns.
        .ignore_then(
            text::ident()
                .then_ignore(just("::"))
                .repeated()
                .collect::<Vec<_>>(),
        )
        .boxed();

    let single_import = text::ident();
    let multi_import = text::ident()
        .separated_by(just(',').padded())
        .at_least(1)
        .collect::<Vec<_>>()
        .delimited_by(just('{').padded(), just('}').padded());
    let blanket_import = just('*').ignored();

    choice((
        path.clone().then(single_import).map(single_entity_id),
        path.clone().then(multi_import).map(multi_entity_id),
        path.then(blanket_import).map(blanket_entity_id),
    ))
    .then_ignore(just(';'))
}

fn single_entity_id(parsed: (Vec<&str>, &str)) -> Import {
    let (mut path, name) = parsed;
    path.push(name);
    Import::Single(EntityId::new_unqualified_vec(path.iter()))
}

fn multi_entity_id(parsed: (Vec<&str>, Vec<&str>)) -> Import {
    let (path, names) = parsed;
    let entity_ids = names
        .into_iter()
        .map(|name| EntityId::new_unqualified_vec(path.iter().chain(std::iter::once(&name))))
        .collect_vec();
    Import::Multi(entity_ids)
}

fn blanket_entity_id(parsed: (Vec<&str>, ())) -> Import {
    let (path, _) = parsed;
    Import::Blanket(EntityId::new_unqualified_vec(path.into_iter()))
}

#[cfg(test)]
mod tests {
    use crate::model::EntityId;
    use crate::parser::rust::import;
    use crate::parser::rust::import::Import;
    use chumsky::Parser;

    #[test]
    fn private() {
        let result = import::parser().parse("use dto;").into_result().unwrap();
        assert_eq!(result, Import::Single(EntityId::new_unqualified("dto")));
    }

    #[test]
    fn public() {
        let result = import::parser()
            .parse("pub use dto;")
            .into_result()
            .unwrap();
        assert_eq!(result, Import::Single(EntityId::new_unqualified("dto")));
    }

    #[test]
    fn ignore_crate() {
        let result = import::parser()
            .parse("pub use crate::dto;")
            .into_result()
            .unwrap();
        assert_eq!(result, Import::Single(EntityId::new_unqualified("dto")));
    }

    #[test]
    fn namespaced() {
        let result = import::parser()
            .parse("pub use a::b::c::d::dto;")
            .into_result()
            .unwrap();
        assert_eq!(
            result,
            Import::Single(EntityId::new_unqualified("a.b.c.d.dto"))
        );
    }

    #[test]
    fn multi() {
        let result = import::parser()
            .parse("use a::b::c::{asd, efg, xyz};")
            .into_result()
            .unwrap();
        assert_eq!(
            result,
            Import::Multi(vec![
                EntityId::new_unqualified("a.b.c.asd"),
                EntityId::new_unqualified("a.b.c.efg"),
                EntityId::new_unqualified("a.b.c.xyz"),
            ])
        );
    }

    #[test]
    fn blanket() {
        let result = import::parser()
            .parse("use a::b::c::*;")
            .into_result()
            .unwrap();
        assert_eq!(result, Import::Blanket(EntityId::new_unqualified("a.b.c")));
    }
}
