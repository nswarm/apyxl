use crate::model::{Attributes, Dto, Field};
use crate::parser::rust::visibility::Visibility;
use crate::parser::rust::{attributes, visibility, Error};
use crate::parser::{comment, rust, Config};
use chumsky::prelude::just;
use chumsky::{text, IterParser, Parser};

pub fn parser(config: &Config) -> impl Parser<&str, (Dto, Visibility), Error> {
    let prefix = rust::keyword_ex("struct").then(text::whitespace().at_least(1));
    let name = text::ident();
    comment::multi_comment()
        .padded()
        .then(attributes::attributes().padded())
        .then(visibility::parser())
        .then_ignore(prefix)
        .then(name)
        .then(fields(config))
        .map(|((((comments, user), visibility), name), fields)| {
            (
                Dto {
                    name,
                    fields,
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

fn fields(config: &Config) -> impl Parser<&str, Vec<Field>, Error> {
    rust::field(config)
        .separated_by(just(',').padded())
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(just('{').padded(), just('}').padded())
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use chumsky::Parser;

    use crate::model::{attribute, Comment};
    use crate::parser::rust::dto;
    use crate::parser::rust::visibility::Visibility;
    use crate::parser::test_util::wrap_test_err;
    use crate::test_util::executor::TEST_CONFIG;

    #[test]
    fn private() -> Result<()> {
        let (dto, visibility) = dto::parser(&TEST_CONFIG)
            .parse(
                r#"
            struct StructName {}
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(dto.name, "StructName");
        assert_eq!(visibility, Visibility::Private);
        Ok(())
    }

    #[test]
    fn public() -> Result<()> {
        let (dto, visibility) = dto::parser(&TEST_CONFIG)
            .parse(
                r#"
            pub struct StructName {}
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(dto.name, "StructName");
        assert_eq!(dto.fields.len(), 0);
        assert_eq!(visibility, Visibility::Public);
        Ok(())
    }

    #[test]
    fn empty() -> Result<()> {
        let (dto, _) = dto::parser(&TEST_CONFIG)
            .parse(
                r#"
            struct StructName {}
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(dto.name, "StructName");
        assert_eq!(dto.fields.len(), 0);
        Ok(())
    }

    #[test]
    fn multiple_fields() -> Result<()> {
        let (dto, _) = dto::parser(&TEST_CONFIG)
            .parse(
                r#"
            struct StructName {
                field0: i32,
                field1: f32,
            }
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(dto.name, "StructName");
        assert_eq!(dto.fields.len(), 2);
        assert_eq!(dto.fields[0].name, "field0");
        assert_eq!(dto.fields[1].name, "field1");
        Ok(())
    }

    #[test]
    fn comment() -> Result<()> {
        let (dto, _) = dto::parser(&TEST_CONFIG)
            .parse(
                r#"
            // multi
            // line
            // comment
            struct StructName {}
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(
            dto.attributes.comments,
            vec![Comment::unowned(&["multi", "line", "comment"])]
        );
        Ok(())
    }

    #[test]
    fn fields_with_comments() -> Result<()> {
        let (dto, _) = dto::parser(&TEST_CONFIG)
            .parse(
                r#"
            struct StructName {
                // multi
                // line
                field0: i32, /* comment */ field1: f32,
            }
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(dto.name, "StructName");
        assert_eq!(dto.fields.len(), 2);
        assert_eq!(dto.fields[0].name, "field0");
        assert_eq!(dto.fields[1].name, "field1");
        assert_eq!(
            dto.fields[0].attributes.comments,
            vec![Comment::unowned(&["multi", "line"])]
        );
        assert_eq!(
            dto.fields[1].attributes.comments,
            vec![Comment::unowned(&["comment"])]
        );
        Ok(())
    }

    #[test]
    fn attributes() -> Result<()> {
        let (dto, _) = dto::parser(&TEST_CONFIG)
            .parse(
                r#"
                #[flag1, flag2]
                struct StructName {}
                "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(dto.name, "StructName");
        assert_eq!(
            dto.attributes.user,
            vec![
                attribute::User::new_flag("flag1"),
                attribute::User::new_flag("flag2"),
            ]
        );
        Ok(())
    }
}
