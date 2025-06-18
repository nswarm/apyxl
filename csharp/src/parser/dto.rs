use crate::parser::is_static::is_static;
use crate::parser::visibility::Visibility;
use crate::parser::{attributes, comment, property, visibility};
use crate::parser::{en, field, rpc, util, Config};
use apyxl::model::{Attributes, Dto, Namespace, NamespaceChild};
use apyxl::parser::error::Error;
use chumsky::prelude::*;
use itertools::Itertools;

pub fn parser(config: &Config) -> impl Parser<&str, (Dto, Visibility), Error> {
    recursive(|nested| {
        let prefix = choice((
            util::keyword_ex("struct"),
            util::keyword_ex("class"),
            util::keyword_ex("interface"),
        ))
        .then(text::whitespace().at_least(1));
        let name = text::ident();
        let children = children(config, nested, just('}').ignored())
            .delimited_by(just('{').padded(), just('}').padded())
            .boxed();
        comment::multi()
            .padded()
            .then(attributes::attributes().padded())
            .then(visibility::parser(Visibility::Private))
            .then_ignore(is_static())
            .then_ignore(prefix)
            .then(name)
            .then(children)
            .map(|((((comments, user), visibility), name), children)| {
                let mut namespace = Namespace {
                    children,
                    ..Default::default()
                };
                let (fields, rpcs) = namespace.extract_non_static();

                let namespace = if namespace.children.is_empty() {
                    None
                } else {
                    Some(namespace)
                };

                let dto = Dto {
                    name,
                    fields,
                    rpcs,
                    attributes: Attributes {
                        comments,
                        user,
                        ..Default::default()
                    },
                    namespace,
                };

                (dto, visibility)
            })
            .boxed()
    })
}

fn children<'a>(
    config: &'a Config,
    dto: impl Parser<'a, &'a str, (Dto<'a>, Visibility), Error<'a>>,
    end_delimiter: impl Parser<'a, &'a str, (), Error<'a>>,
) -> impl Parser<'a, &'a str, Vec<NamespaceChild<'a>>, Error<'a>> {
    choice((
        dto.map(|(c, v)| vec![(NamespaceChild::Dto(c), v)]),
        en::parser().map(|(c, v)| vec![(NamespaceChild::Enum(c), v)]),
        rpc::parser(config).map(|(c, v)| vec![(NamespaceChild::Rpc(c), v)]),
        property::parser(config).map(|properties| {
            properties
                .into_iter()
                .map(|(rpc, v)| (NamespaceChild::Rpc(rpc), v))
                .collect_vec()
        }),
        // Field after property so that it can be greedy with '=<whatever>;'.
        field::parser(config).map(|(c, v)| vec![(NamespaceChild::Field(c), v)]),
    ))
    .recover_with(skip_then_retry_until(
        any().ignored(),
        end_delimiter.ignored(),
    ))
    .map(|vec| {
        vec.into_iter()
            .filter_map(|(child, visibility)| visibility.filter(child, config))
            .collect_vec()
    })
    .repeated()
    .collect::<Vec<_>>()
    .map(|v| v.into_iter().flatten().collect_vec())
    .then_ignore(comment::multi())
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use chumsky::Parser;

    use crate::parser::dto;
    use crate::parser::visibility::Visibility;
    use apyxl::model::{attributes, Comment};
    use apyxl::parser::test_util::wrap_test_err;
    use apyxl::test_util::executor::{TEST_CONFIG, TEST_PUB_ONLY_CONFIG};

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
    fn public_static() -> Result<()> {
        let (dto, visibility) = dto::parser(&TEST_CONFIG)
            .parse(
                r#"
            public static class ClassName {}
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(dto.name, "ClassName");
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
    fn non_static_field() -> Result<()> {
        let (dto, _) = dto::parser(&TEST_CONFIG)
            .parse(
                r#"
            struct StructName {
                int field0 = 5;
            }
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(dto.name, "StructName");
        assert_eq!(dto.fields.len(), 1);
        assert!(dto.field("field0").is_some());
        assert!(!dto.field("field0").unwrap().is_static);
        Ok(())
    }

    #[test]
    fn static_field() -> Result<()> {
        let (dto, _) = dto::parser(&TEST_CONFIG)
            .parse(
                r#"
            struct StructName {
                static int field0 = 5;
            }
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(dto.name, "StructName");
        assert_eq!(dto.fields.len(), 0);
        assert!(dto.namespace.is_some());
        let namespace = dto.namespace.unwrap();
        assert_eq!(namespace.children.len(), 1);
        assert!(namespace.field("field0").is_some());
        assert!(namespace.field("field0").unwrap().is_static);
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
    fn initializers() -> Result<()> {
        let (dto, _) = dto::parser(&TEST_CONFIG)
            .parse(
                r#"
                struct StructName {
                    int field0 = 1;
                    string field1 = "asbcd";
                    string field2 = SomeSuper.Complex().default("var");
                    public static string STRING = "blah";
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
        assert_eq!(dto.namespace.unwrap().children[0].name(), "STRING");
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
        assert_eq!(dto.rpcs.len(), 2);
        assert!(dto.rpc("rpc").is_some());
        assert!(dto.rpc("other_rpc").is_some());
        Ok(())
    }

    #[test]
    fn static_rpc() -> Result<()> {
        let (dto, _) = dto::parser(&TEST_CONFIG)
            .parse(
                r#"
                struct StructName {
                    private static void rpc() {}
                    public static int other_rpc() {}
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
        assert!(namespace.rpc("rpc").unwrap().is_static);
        assert!(namespace.rpc("other_rpc").is_some());
        assert!(namespace.rpc("other_rpc").unwrap().is_static);
        Ok(())
    }

    #[test]
    fn property() -> Result<()> {
        let (dto, _) = dto::parser(&TEST_CONFIG)
            .parse(
                r#"
                struct StructName {
                    int prop { get; set; }
                }
                "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(dto.name, "StructName");
        assert_eq!(dto.rpcs.len(), 2);
        assert!(dto.rpc("get_prop").is_some());
        assert!(dto.rpc("set_prop").is_some());
        Ok(())
    }

    #[test]
    fn static_property() -> Result<()> {
        let (dto, _) = dto::parser(&TEST_CONFIG)
            .parse(
                r#"
                struct StructName {
                    static int prop { get; set; }
                }
                "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(dto.name, "StructName");
        assert!(dto.namespace.is_some());
        let namespace = dto.namespace.as_ref().unwrap();
        assert_eq!(namespace.children.len(), 2);
        assert!(namespace.rpc("get_prop").is_some());
        assert!(namespace.rpc("get_prop").unwrap().is_static);
        assert!(namespace.rpc("set_prop").is_some());
        assert!(namespace.rpc("set_prop").unwrap().is_static);
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
        assert!(dto.rpc("rpc").is_some());
        assert!(dto.rpc("other_rpc").is_some());
        Ok(())
    }

    #[test]
    fn mixed_field_property() -> Result<()> {
        let (dto, _) = dto::parser(&TEST_CONFIG)
            .parse(
                r#"
                struct StructName {
                    int field0 = 1;
                    string field1 => "asbcd";
                    string field2 {
                        get => "zzz";
                    }
                }
                "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(dto.name, "StructName");
        println!("FIELDS\n{:#?}", dto.fields);
        println!("\n\nRPCS{:#?}", dto.rpcs);
        assert_eq!(dto.fields.len(), 1);
        assert_eq!(dto.fields[0].name, "field0");
        assert_eq!(dto.rpcs.len(), 2);
        assert_eq!(dto.rpcs[0].name, "get_field1");
        assert_eq!(dto.rpcs[1].name, "get_field2");
        Ok(())
    }

    #[test]
    fn nested_enum() -> Result<()> {
        let (dto, _) = dto::parser(&TEST_CONFIG)
            .parse(
                r#"
            struct StructName {
                enum en {}
            }
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert!(dto.namespace.is_some());
        let namespace = dto.namespace.as_ref().unwrap();
        assert!(namespace.en("en").is_some());
        Ok(())
    }

    #[test]
    fn nested_class() -> Result<()> {
        let (dto, _) = dto::parser(&TEST_CONFIG)
            .parse(
                r#"
            struct StructName {
                class Nested {
                    class Nested2 {}
                }
            }
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert!(dto.namespace.is_some());
        let namespace = dto.namespace.as_ref().unwrap();
        assert!(namespace.dto("Nested").is_some());
        let namespace = namespace.dto("Nested").unwrap().namespace.as_ref().unwrap();
        assert!(namespace.dto("Nested2").is_some());
        Ok(())
    }

    #[test]
    fn no_nested_namespace() {
        let result = dto::parser(&TEST_CONFIG)
            .parse(
                r#"
            struct StructName {
                namespace blah {}
            }
            "#,
            )
            .into_result();
        assert!(result.is_err());
    }
}
