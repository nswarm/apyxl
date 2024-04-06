use chumsky::prelude::*;

use crate::model::{Attributes, TypeAlias};
use crate::parser::error::Error;
use crate::parser::rust::visibility::Visibility;
use crate::parser::rust::{attributes, ty, visibility};
use crate::parser::{comment, util, Config};

pub fn parser(config: &Config) -> impl Parser<&str, (TypeAlias, Visibility), Error> {
    let prefix = util::keyword_ex("type").then(text::whitespace().at_least(1));
    comment::multi_comment()
        .padded()
        .then(attributes::attributes().padded())
        .then(visibility::parser())
        .then_ignore(prefix)
        .then(text::ident())
        .then_ignore(just("=").padded())
        .then(ty::parser(config))
        .then_ignore(just(';'))
        .padded()
        .map(|((((comments, user), visibility), name), target)| {
            (
                TypeAlias {
                    name,
                    target_ty: target,
                    attributes: Attributes {
                        comments,
                        user,
                        ..Default::default()
                    },
                },
                visibility,
            )
        })
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use chumsky::Parser;

    use crate::model::{EntityId, Semantics, Type, TypeRef};
    use crate::parser::rust::ty_alias;
    use crate::parser::rust::visibility::Visibility;
    use crate::parser::test_util::wrap_test_err;
    use crate::test_util::executor::TEST_CONFIG;

    #[test]
    fn public() -> Result<()> {
        let (alias, visibility) = ty_alias::parser(&TEST_CONFIG)
            .parse("pub type AliasName = pkg::SomeType;")
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(alias.name, "AliasName");
        assert_eq!(visibility, Visibility::Public);
        assert_eq!(
            alias.target_ty,
            TypeRef::new(
                Type::Api(EntityId::new_unqualified("pkg.SomeType")),
                Semantics::Value
            )
        );
        Ok(())
    }

    #[test]
    fn private() -> Result<()> {
        let (alias, visibility) = ty_alias::parser(&TEST_CONFIG)
            .parse("type OtherName = u32;")
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(alias.name, "OtherName");
        assert_eq!(visibility, Visibility::Private);
        assert_eq!(alias.target_ty, TypeRef::new(Type::U32, Semantics::Value));
        Ok(())
    }
}
