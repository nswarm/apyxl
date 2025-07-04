use std::borrow::Cow;

use chumsky::prelude::*;
use itertools::Itertools;

use crate::model::{Attributes, Field, Namespace, NamespaceChild};
use crate::parser::error::Error;
use crate::parser::rust::visibility::Visibility;
use crate::parser::rust::{attributes, comment, dto, en, rpc, ty, ty_alias, visibility};
use crate::parser::{util, Config};

pub fn parser(config: &Config) -> impl Parser<&str, (Namespace, Visibility), Error> {
    recursive(|nested| {
        let prefix = util::keyword_ex("mod").then(text::whitespace().at_least(1));
        let name = text::ident();
        let body = children(config, nested.clone(), just('}').ignored())
            .delimited_by(just('{').padded(), just('}').padded());
        comment::multi()
            .then(attributes::attributes().padded())
            .then(visibility::parser())
            .then_ignore(prefix)
            .then(name)
            .then(just(';').padded().map(|_| None).or(body.map(Some)))
            .map(|((((comments, user), visibility), name), children)| {
                (
                    Namespace {
                        name: Cow::Borrowed(name),
                        children: children.unwrap_or(vec![]),
                        attributes: Attributes {
                            comments,
                            user,
                            ..Default::default()
                        },
                        is_virtual: false,
                    },
                    visibility,
                )
            })
            .boxed()
    })
}

pub fn children<'a>(
    config: &'a Config,
    namespace: impl Parser<'a, &'a str, (Namespace<'a>, Visibility), Error<'a>>,
    end_delimiter: impl Parser<'a, &'a str, (), Error<'a>>,
) -> impl Parser<'a, &'a str, Vec<NamespaceChild<'a>>, Error<'a>> {
    choice((
        dto::parser(config).map(|(c, v)| Some((NamespaceChild::Dto(c), v))),
        rpc::parser(config).map(|(c, v)| Some((NamespaceChild::Rpc(c), v))),
        en::parser().map(|(c, v)| Some((NamespaceChild::Enum(c), v))),
        ty_alias::parser(config).map(|(c, v)| Some((NamespaceChild::TypeAlias(c), v))),
        field(config).map(|(c, v)| Some((NamespaceChild::Field(c), v))),
        namespace.map(|(c, v)| Some((NamespaceChild::Namespace(c), v))),
        impl_block(config).map(|c| Some((NamespaceChild::Namespace(c), Visibility::Public))),
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

fn field(config: &Config) -> impl Parser<&str, (Field, Visibility), Error> {
    let end = just(';');
    let initializer = just('=')
        .padded()
        .then(any().and_is(end.not()).repeated().slice());
    let field = util::keyword_ex("const")
        .ignore_then(text::whitespace().at_least(1))
        .ignore_then(text::ident())
        .then_ignore(just(':').padded())
        .then(ty::parser(config))
        .then_ignore(initializer)
        .then_ignore(end.padded());
    comment::multi()
        .then(attributes::attributes().padded())
        .then(visibility::parser())
        .then(field)
        .map(|(((comments, user), visibility), (name, ty))| {
            (
                Field {
                    name,
                    ty,
                    attributes: Attributes {
                        comments,
                        user,
                        ..Default::default()
                    },
                    is_static: true,
                },
                visibility,
            )
        })
}

// Parses to a 'virtual' namespace that will be merged into the DTO with the same name.
pub fn impl_block(config: &Config) -> impl Parser<&str, Namespace, Error> {
    let prefix = util::keyword_ex("impl").then(text::whitespace().at_least(1));

    let children = choice((
        rpc::parser(config).map(|(c, v)| Some((NamespaceChild::Rpc(c), v))),
        ty_alias::parser(config).map(|(c, v)| Some((NamespaceChild::TypeAlias(c), v))),
        field(config).map(|(c, v)| Some((NamespaceChild::Field(c), v))),
    ))
    .recover_with(skip_then_retry_until(any().ignored(), just('}').ignored()))
    .map(|opt| match opt {
        Some((child, visibility)) => visibility.filter(child, config),
        None => None,
    })
    .repeated()
    .collect::<Vec<_>>()
    .map(|v| v.into_iter().flatten().collect_vec());

    comment::multi()
        .padded()
        .then_ignore(prefix)
        .then(text::ident())
        .then(children.delimited_by(just('{').padded(), just('}').padded()))
        .map(|((comments, name), children)| Namespace {
            name: Cow::Borrowed(name),
            children,
            attributes: Attributes {
                comments,
                ..Default::default()
            },
            is_virtual: true,
        })
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use chumsky::Parser;

    use crate::model::{attributes, Comment, NamespaceChild};
    use crate::parser::rust::namespace;
    use crate::parser::rust::visibility::Visibility;
    use crate::parser::test_util::wrap_test_err;
    use crate::test_util::executor::TEST_CONFIG;

    #[test]
    fn declaration() -> Result<()> {
        let (namespace, _) = namespace::parser(&TEST_CONFIG)
            .parse(
                r#"
            mod empty;
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(namespace.name, "empty");
        assert!(namespace.children.is_empty());
        Ok(())
    }

    #[test]
    fn public() -> Result<()> {
        let (namespace, visibility) = namespace::parser(&TEST_CONFIG)
            .parse(
                r#"
            pub mod empty;
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(namespace.name, "empty");
        assert_eq!(visibility, Visibility::Public);
        Ok(())
    }

    #[test]
    fn private() -> Result<()> {
        let (namespace, visibility) = namespace::parser(&TEST_CONFIG)
            .parse(
                r#"
            mod empty;
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(namespace.name, "empty");
        assert_eq!(visibility, Visibility::Private);
        Ok(())
    }

    #[test]
    fn empty() -> Result<()> {
        let (namespace, _) = namespace::parser(&TEST_CONFIG)
            .parse(
                r#"
            mod empty {}
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(namespace.name, "empty");
        assert!(namespace.children.is_empty());
        Ok(())
    }

    #[test]
    fn with_dto() -> Result<()> {
        let (namespace, _) = namespace::parser(&TEST_CONFIG)
            .parse(
                r#"
            mod ns {
                pub struct DtoName {}
            }
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(namespace.name, "ns");
        assert_eq!(namespace.children.len(), 1);
        match &namespace.children[0] {
            NamespaceChild::Dto(dto) => assert_eq!(dto.name, "DtoName"),
            _ => panic!("wrong child type"),
        }
        Ok(())
    }

    #[test]
    fn with_field() -> Result<()> {
        let (namespace, _) = namespace::parser(&TEST_CONFIG)
            .parse(
                r#"
            mod ns {
                // comment
                // comment
                #[attr]
                pub const field0: &str = "blahh";
                // comment
                const field1: &str = "blahh";
            }
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(namespace.name, "ns");
        assert_eq!(namespace.children.len(), 2);
        match &namespace.children[0] {
            NamespaceChild::Field(field) => assert_eq!(field.name, "field0"),
            _ => panic!("wrong child type"),
        }
        match &namespace.children[1] {
            NamespaceChild::Field(field) => assert_eq!(field.name, "field1"),
            _ => panic!("wrong child type"),
        }
        Ok(())
    }

    #[test]
    fn nested() -> Result<()> {
        let (namespace, _) = namespace::parser(&TEST_CONFIG)
            .parse(
                r#"
            mod ns0 {
                mod ns1 {}
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
    fn nested_dto() -> Result<()> {
        let (namespace, _) = namespace::parser(&TEST_CONFIG)
            .parse(
                r#"
            mod ns0 {
                mod ns1 {
                    pub struct DtoName {}
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
        let (namespace, _) = namespace::parser(&TEST_CONFIG)
            .parse(
                r#"
            // multi
            // line
            // comment
            mod ns {}
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
                    mod ns { // comment
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
                    mod ns { /* comment */
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
        let (namespace, _) = namespace::parser(&TEST_CONFIG)
            .parse(
                r#"
                    #[flag1, flag2]
                    mod ns {}
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

    #[test]
    fn impl_block() -> Result<()> {
        let namespace = namespace::impl_block(&TEST_CONFIG)
            .parse(
                r#"
                    impl dto {
                        const x: Type = 5;
                        type T = V;
                        fn rpc() {}
                    }
                    "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert!(namespace.is_virtual);
        assert!(namespace.field("x").is_some());
        assert!(namespace.rpc("rpc").is_some());
        assert!(namespace.ty_alias("T").is_some());
        Ok(())
    }

    #[test]
    fn impl_block_nested() -> Result<()> {
        let (namespace, _) = namespace::parser(&TEST_CONFIG)
            .parse(
                r#"
                    mod ns {
                        impl dto {
                            fn rpc() {}
                        }
                    }
                    "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert!(!namespace.is_virtual);
        let dto_ns = namespace.namespace("dto");
        assert!(dto_ns.is_some());
        let dto_ns = dto_ns.unwrap();
        assert!(dto_ns.is_virtual);
        assert!(dto_ns.rpc("rpc").is_some());
        Ok(())
    }

    #[test]
    fn pub_field() -> Result<()> {
        let (field, visibility) = namespace::field(&TEST_CONFIG)
            .parse(r#"pub const field0: &str = "blahh";"#)
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(field.name, "field0");
        assert!(field.is_static);
        assert!(matches!(visibility, Visibility::Public));
        Ok(())
    }

    #[test]
    fn private_field() -> Result<()> {
        let (field, visibility) = namespace::field(&TEST_CONFIG)
            .parse(r#"const field0: &str = "blahh";"#)
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(field.name, "field0");
        assert!(field.is_static);
        assert!(matches!(visibility, Visibility::Private));
        Ok(())
    }
}
