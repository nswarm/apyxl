use std::borrow::Cow;

use chumsky::prelude::*;
use itertools::Itertools;

use crate::parser::error::Error;
use crate::parser::util::keyword_ex;
use crate::parser::visibility::Visibility;
use crate::parser::{attributes, comment, dto, en, ty_alias};
use apyxl::model::{Attributes, Namespace, NamespaceChild};
use apyxl::parser::Config;

pub fn parser(config: &Config) -> impl Parser<&str, Namespace, Error> {
    recursive(|nested| {
        let prefix = keyword_ex("namespace").then(text::whitespace().at_least(1));
        let name_chain = text::ident()
            .separated_by(just("."))
            .at_least(1)
            .collect::<Vec<_>>();
        let body = children(config, nested.clone(), just('}').ignored())
            .delimited_by(just('{').padded(), just('}').padded());
        comment::multi()
            .then(attributes::attributes().padded())
            .then_ignore(prefix)
            .then(name_chain)
            .then(body)
            .map(|(((comments, user), mut name_chain), children)| {
                let name = name_chain.remove(name_chain.len() - 1);
                let mut namespace = Namespace {
                    name: Cow::Borrowed(name),
                    children,
                    attributes: Attributes {
                        comments,
                        user,
                        ..Default::default()
                    },
                    is_virtual: false,
                };
                // For inline nested namespaces e.g. `namespace a.b.c`, walk the name_chain
                // in reverse, and wrapping each level in a new namespace.
                for parent in name_chain.into_iter().rev() {
                    namespace = Namespace {
                        name: Cow::Borrowed(parent),
                        children: vec![NamespaceChild::Namespace(namespace)],
                        ..Default::default()
                    }
                }
                namespace
            })
            .boxed()
    })
}

pub fn children<'a>(
    config: &'a Config,
    namespace: impl Parser<'a, &'a str, Namespace<'a>, Error<'a>>,
    end_delimiter: impl Parser<'a, &'a str, (), Error<'a>>,
) -> impl Parser<'a, &'a str, Vec<NamespaceChild<'a>>, Error<'a>> {
    choice((
        dto::parser(config).map(|(c, v)| Some((NamespaceChild::Dto(c), v))),
        en::parser().map(|(c, v)| Some((NamespaceChild::Enum(c), v))),
        ty_alias::parser(config)
            .map(|c| c.map(|alias| (NamespaceChild::TypeAlias(alias), Visibility::Public))),
        namespace.map(|c| Some((NamespaceChild::Namespace(c), Visibility::Public))),
    ))
    .recover_with(skip_then_retry_until(
        any().ignored(),
        end_delimiter.ignored(),
    ))
    .map(|opt| match opt {
        Some((child, visibility)) => visibility.filter(child, config),
        None => None,
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

    use crate::parser::namespace;
    use apyxl::model::{attributes, Comment, NamespaceChild};
    use apyxl::parser::test_util::wrap_test_err;
    use apyxl::test_util::executor::TEST_CONFIG;

    #[test]
    fn declaration() -> Result<()> {
        let namespace = namespace::parser(&TEST_CONFIG)
            .parse(
                r#"
            namespace empty {}
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(namespace.name, "empty");
        assert!(namespace.children.is_empty());
        Ok(())
    }

    #[test]
    fn empty() -> Result<()> {
        let namespace = namespace::parser(&TEST_CONFIG)
            .parse(
                r#"
            namespace empty {}
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(namespace.name, "empty");
        assert!(namespace.children.is_empty());
        Ok(())
    }

    #[test]
    fn with_children() -> Result<()> {
        let namespace = namespace::parser(&TEST_CONFIG)
            .parse(
                r#"
            namespace ns {
                using Type = int;
                public struct DtoName {}
                public enum Enum {}
            }
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(namespace.name, "ns");
        assert_eq!(namespace.children.len(), 3);
        assert!(namespace.ty_alias("Type").is_some());
        assert!(namespace.dto("DtoName").is_some());
        assert!(namespace.en("Enum").is_some());
        Ok(())
    }

    #[test]
    fn no_field_children() {
        let result = namespace::parser(&TEST_CONFIG)
            .parse(
                r#"
            namespace ns {
                public static int field = 5;
            }
            "#,
            )
            .into_result();
        assert!(result.is_err());
    }

    #[test]
    fn no_rpc_children() {
        let result = namespace::parser(&TEST_CONFIG)
            .parse(
                r#"
            namespace ns {
                public void Func() {}
            }
            "#,
            )
            .into_result();
        assert!(result.is_err());
    }

    #[test]
    fn nested() -> Result<()> {
        let namespace = namespace::parser(&TEST_CONFIG)
            .parse(
                r#"
            namespace ns0 {
                namespace ns1 {}
            }
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(namespace.name, "ns0");
        assert_eq!(namespace.children.len(), 1);
        match &namespace.children[0] {
            NamespaceChild::Namespace(ns) => assert_eq!(ns.name, "ns1"),
            _ => panic!("wrong child type"),
        }
        Ok(())
    }

    #[test]
    fn nested_inline() -> Result<()> {
        let ns0 = namespace::parser(&TEST_CONFIG)
            .parse(
                r#"
            namespace ns0.ns1.ns2 {}
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(ns0.name, "ns0");
        assert_eq!(ns0.children.len(), 1);

        let ns1 = ns0.namespace("ns1");
        assert!(ns1.is_some());
        let ns1 = ns1.unwrap();
        assert_eq!(ns1.name, "ns1");
        assert_eq!(ns1.children.len(), 1);

        let ns2 = ns1.namespace("ns2");
        assert!(ns2.is_some());
        let ns2 = ns2.unwrap();
        assert_eq!(ns2.name, "ns2");
        assert_eq!(ns2.children.len(), 0);

        Ok(())
    }

    #[test]
    fn nested_dto() -> Result<()> {
        let namespace = namespace::parser(&TEST_CONFIG)
            .parse(
                r#"
            namespace ns0 {
                namespace ns1 {
                    public struct DtoName {}
                }
            }
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(namespace.name, "ns0");
        assert_eq!(namespace.children.len(), 1);
        match &namespace.children[0] {
            NamespaceChild::Namespace(ns) => {
                assert_eq!(ns.name, "ns1");
                assert_eq!(ns.children.len(), 1);
                match &ns.children[0] {
                    NamespaceChild::Dto(dto) => assert_eq!(dto.name, "DtoName"),
                    _ => panic!("ns1: wrong child type"),
                }
            }
            _ => panic!("ns0: wrong child type"),
        }
        Ok(())
    }

    #[test]
    fn comment() -> Result<()> {
        let namespace = namespace::parser(&TEST_CONFIG)
            .parse(
                r#"
            // multi
            // line
            // comment
            namespace ns {}
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(
            namespace.attributes.comments,
            vec![Comment::unowned(&["multi", "line", "comment"])]
        );
        Ok(())
    }

    #[test]
    fn line_comments_inside_namespace() -> Result<()> {
        namespace::parser(&TEST_CONFIG)
            .parse(
                r#"
                    namespace ns { // comment
                        // comment

                        // comment
                        // comment
                        // comment
                        struct dto {} // comment
                        // comment
                    }
                    "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        Ok(())
    }

    #[test]
    fn block_comment_inside_namespace() -> Result<()> {
        namespace::parser(&TEST_CONFIG)
            .parse(
                r#"
                    namespace ns { /* comment */
                        /* comment */
                        /* comment */
                        struct dto {} /* comment */
                        /* comment */
                    }
                    "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        Ok(())
    }

    #[test]
    fn attributes() -> Result<()> {
        let namespace = namespace::parser(&TEST_CONFIG)
            .parse(
                r#"
                    [flag1, flag2]
                    namespace ns {}
                    "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(
            namespace.attributes.user,
            vec![
                attributes::User::new_flag("flag1"),
                attributes::User::new_flag("flag2"),
            ]
        );
        Ok(())
    }
}
