use chumsky::prelude::*;

use crate::parser::{attributes, comment, ty};
use apyxl::model::{Attributes, TypeAlias};
use apyxl::parser::error::Error;
use apyxl::parser::{util, Config};

pub fn parser(config: &Config) -> impl Parser<&str, TypeAlias, Error> {
    let prefix = util::keyword_ex("using").then(text::whitespace().at_least(1));
    let alias_name = text::ident().then_ignore(just("=").padded());
    comment::multi()
        .then(attributes::attributes().padded())
        .then_ignore(prefix)
        .then(alias_name)
        .then(ty::parser(config))
        .then_ignore(just(';').padded())
        .map(|(((comments, user), name), target)| TypeAlias {
            name,
            target_ty: target,
            attributes: Attributes {
                comments,
                user,
                ..Default::default()
            },
        })
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use chumsky::Parser;

    use crate::parser::ty_alias;
    use apyxl::model::{EntityId, Semantics, Type, TypeAlias, TypeRef};
    use apyxl::parser::test_util::wrap_test_err;
    use apyxl::test_util::executor::TEST_CONFIG;

    #[test]
    fn alias() -> Result<()> {
        let alias = parse_alias(r#"using a = b;"#)?;
        assert_eq!(alias.name, "a");
        assert_eq!(alias.target_ty, type_ref("b"));
        Ok(())
    }

    #[test]
    fn complex_alias() -> Result<()> {
        let alias = parse_alias(r#"using abc_def = a.b.c.d;"#)?;
        assert_eq!(alias.name, "abc_def");
        assert_eq!(alias.target_ty, type_ref("a.b.c.d"));
        Ok(())
    }

    fn type_ref(id: &str) -> TypeRef {
        TypeRef {
            value: Type::Api(EntityId::new_unqualified(id)),
            semantics: Semantics::Value,
        }
    }

    fn parse_alias(input: &'static str) -> Result<TypeAlias<'static>> {
        let result = ty_alias::parser(&TEST_CONFIG)
            .parse(input)
            .into_result()
            .map_err(wrap_test_err)?;
        Ok(result)
    }
}
