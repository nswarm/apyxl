use chumsky::prelude::*;

use crate::parser::{attributes, comment, ty};
use apyxl::model::{Attributes, TypeAlias};
use apyxl::parser::error::Error;
use apyxl::parser::{util, Config};

pub fn parser(config: &Config) -> impl Parser<&str, Option<TypeAlias>, Error> {
    let prefix = util::keyword_ex("using").then(text::whitespace().at_least(1));
    let alias_name = text::ident().then_ignore(just("=").padded());
    comment::multi()
        .then(attributes::attributes().padded())
        .then_ignore(prefix)
        .then(alias_name.or_not())
        .then(ty::parser(config))
        .then_ignore(just(';').padded())
        .map(|(((comments, user), alias_name), target)| {
            // todo.... namespace usings need to be considered when qualifying types
            alias_name.map(|name| TypeAlias {
                name,
                target_ty: target,
                attributes: Attributes {
                    comments,
                    user,
                    ..Default::default()
                },
            })
        })
}

#[cfg(test)]
mod tests {
    use anyhow::{anyhow, Result};
    use chumsky::Parser;

    use crate::parser::ty_alias;
    use apyxl::model::{EntityId, Semantics, Type, TypeAlias, TypeRef};
    use apyxl::parser::test_util::wrap_test_err;
    use apyxl::test_util::executor::TEST_CONFIG;

    #[test]
    fn basic_import() -> Result<()> {
        parse_import("using asd;")
    }

    #[test]
    fn namespaced_using() -> Result<()> {
        parse_import("using a.b.c.d;")
    }

    #[test]
    fn using_with_comment() -> Result<()> {
        parse_import(
            r#"
            // comment
            using a.b.c.d;
            "#,
        )
    }

    #[test]
    fn using_with_attrs() -> Result<()> {
        parse_import(
            r#"
            [attr]
            using a.b.c.d;
            "#,
        )
    }

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
            .map_err(wrap_test_err)?
            .ok_or(anyhow!("not an alias"))?;
        Ok(result)
    }

    fn parse_import(input: &'static str) -> Result<()> {
        let result = ty_alias::parser(&TEST_CONFIG)
            .parse(input)
            .into_result()
            .map_err(wrap_test_err)?;
        if result.is_some() {
            Err(anyhow!("import should result in None alias"))
        } else {
            Ok(())
        }
    }
}
