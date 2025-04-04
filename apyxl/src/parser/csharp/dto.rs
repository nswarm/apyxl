use chumsky::prelude::*;
use itertools::Itertools;

use crate::model::{Attributes, Dto, Field, Namespace, Rpc};
use crate::parser::csharp::visibility::Visibility;
use crate::parser::csharp::{attributes, comment, rpc, ty, visibility};
use crate::parser::error::Error;
use crate::parser::{util, Config};

// todo maybe just use NamespaceChild?
enum DtoChild<'a> {
    Field(Field<'a>),
    Rpc(Rpc<'a>),
}

pub fn parser(config: &Config) -> impl Parser<&str, (Dto, Visibility), Error> {
    let prefix = choice((
        // todo handle differently?
        util::keyword_ex("struct"),
        util::keyword_ex("class"),
        util::keyword_ex("interface"),
    ))
    .then(text::whitespace().at_least(1));
    let name = text::ident();
    let child = choice((
        field(config).map(|(field, v)| (DtoChild::Field(field), v)),
        rpc::parser(config).map(|(rpc, v)| (DtoChild::Rpc(rpc), v)),
    ));
    let children = child.repeated().collect::<Vec<_>>().delimited_by(
        just('{').padded(),
        just('}').padded().recover_with(skip_then_retry_until(
            none_of("}").ignored(),
            just('}').ignored(),
        )),
    );
    comment::multi()
        .padded()
        .then(attributes::attributes().padded())
        .then(visibility::parser())
        .then_ignore(prefix)
        .then(name)
        .then(children)
        .map(|((((comments, user), visibility), name), children)| {
            let (fields, rpcs): (Vec<DtoChild>, Vec<DtoChild>) = children
                .into_iter()
                .filter_map(|(child, visibility)| visibility.filter(child, config))
                .partition(|child| match child {
                    DtoChild::Field(_) => true,
                    DtoChild::Rpc(_) => false,
                });
            let fields = fields
                .into_iter()
                .filter_map(|child| match child {
                    DtoChild::Field(field) => Some(field),
                    _ => None,
                })
                .collect_vec();
            let mut namespace = Namespace::default();
            rpcs.into_iter()
                .filter_map(|child| match child {
                    DtoChild::Rpc(rpc) => Some(rpc),
                    _ => None,
                })
                .for_each(|rpc| namespace.add_rpc(rpc));
            (
                Dto {
                    name,
                    fields,
                    attributes: Attributes {
                        comments,
                        user,
                        ..Default::default()
                    },
                    namespace: Some(namespace),
                },
                visibility,
            )
        })
}

fn field(config: &Config) -> impl Parser<&str, (Field, Visibility), Error> {
    let end = just(';');
    let initializer = just('=')
        .padded()
        .then(any().and_is(end.not()).repeated().slice());
    let field = ty::parser(config)
        .then_ignore(text::whitespace().at_least(1))
        .then(text::ident())
        .then_ignore(initializer.or_not())
        .then_ignore(end.padded());
    // todo properties
    // todo events
    comment::multi()
        .then(attributes::attributes().padded())
        .then(visibility::parser())
        .then_ignore(util::keyword_ex("readonly").or_not())
        .then(field)
        .map(|(((comments, user), visibility), (ty, name))| {
            (
                Field {
                    name,
                    ty,
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

    use crate::model::{attributes, Comment};
    use crate::parser::csharp::dto;
    use crate::parser::csharp::visibility::Visibility;
    use crate::parser::test_util::wrap_test_err;
    use crate::test_util::executor::{TEST_CONFIG, TEST_PUB_ONLY_CONFIG};

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
            public struct StructName {}
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
                int field0;
                public float field1;
                float field2;
            }
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(dto.name, "StructName");
        assert_eq!(dto.fields.len(), 3);
        assert_eq!(dto.fields[0].name, "field0");
        assert_eq!(dto.fields[1].name, "field1");
        assert_eq!(dto.fields[2].name, "field2");
        Ok(())
    }

    #[test]
    fn field_visibility() -> Result<()> {
        let (dto, _) = dto::parser(&TEST_PUB_ONLY_CONFIG)
            .parse(
                r#"
            struct StructName {
                int field0;
                public float field1;
                float field2;
            }
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(dto.name, "StructName");
        assert_eq!(dto.fields.len(), 1);
        assert_eq!(dto.fields[0].name, "field1");
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
                int field0; /* comment */ float field1;
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
                [flag1, flag2]
                struct StructName {}
                "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(dto.name, "StructName");
        assert_eq!(
            dto.attributes.user,
            vec![
                attributes::User::new_flag("flag1"),
                attributes::User::new_flag("flag2"),
            ]
        );
        Ok(())
    }

    #[test]
    fn initializers() -> Result<()> {
        let (dto, _) = dto::parser(&TEST_CONFIG)
            .parse(
                r#"
                struct StructName {
                    int field0 = 1;
                    string field1 = "asbcd";
                    string field2 = SomeSuper.Complex().default("var");
                }
                "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(dto.name, "StructName");
        assert_eq!(dto.fields.len(), 3);
        assert_eq!(dto.fields[0].name, "field0");
        assert_eq!(dto.fields[1].name, "field1");
        assert_eq!(dto.fields[2].name, "field2");
        Ok(())
    }

    #[test]
    fn rpc() -> Result<()> {
        let (dto, _) = dto::parser(&TEST_CONFIG)
            .parse(
                r#"
                struct StructName {
                    private void rpc() {}
                    public int other_rpc() {}
                }
                "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(dto.name, "StructName");
        assert!(dto.namespace.is_some());
        let namespace = dto.namespace.as_ref().unwrap();
        assert_eq!(namespace.children.len(), 2);
        assert!(namespace.rpc("rpc").is_some());
        assert!(namespace.rpc("other_rpc").is_some());
        Ok(())
    }

    #[test]
    fn mixed_rpc_dto() -> Result<()> {
        let (dto, _) = dto::parser(&TEST_CONFIG)
            .parse(
                r#"
                struct StructName {
                    private void rpc() {}
                    int field0 = 1;
                    string field1 = "asbcd";
                    public int other_rpc() {}
                    string field2 = SomeSuper.Complex().default("var");
                }
                "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(dto.name, "StructName");
        assert_eq!(dto.fields.len(), 3);
        assert_eq!(dto.fields[0].name, "field0");
        assert_eq!(dto.fields[1].name, "field1");
        assert_eq!(dto.fields[2].name, "field2");
        assert!(dto.namespace.is_some());
        let namespace = dto.namespace.as_ref().unwrap();
        assert_eq!(namespace.children.len(), 2);
        assert!(namespace.rpc("rpc").is_some());
        assert!(namespace.rpc("other_rpc").is_some());
        Ok(())
    }

    #[test]
    fn nested_enum() -> Result<()> {
        todo!("nyi")
    }

    #[test]
    fn nested_class() -> Result<()> {
        todo!("nyi")
    }

    #[test]
    fn complex_nested() -> Result<()> {
        todo!("nyi")
    }
}
