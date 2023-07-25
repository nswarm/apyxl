use std::borrow::Cow;

use chumsky::prelude::*;
use chumsky::{text, IterParser, Parser};
use itertools::Itertools;

use crate::model::{Attributes, Namespace, NamespaceChild};
use crate::parser::error::Error;
use crate::parser::rust::visibility::Visibility;
use crate::parser::rust::{attributes, dto, en, rpc, visibility};
use crate::parser::util::keyword_ex;
use crate::parser::{comment, Config};

pub fn parser(config: &Config) -> impl Parser<&str, (Namespace, Visibility), Error> {
    recursive(|nested| {
        let prefix = keyword_ex("mod").then(text::whitespace().at_least(1));
        let name = text::ident();
        let body = children(
            config,
            nested.clone(),
            impl_block(config, nested),
            just('}').ignored(),
        )
        .delimited_by(just('{').padded(), just('}').padded());
        comment::multi_comment()
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
    impl_block: impl Parser<'a, &'a str, Namespace<'a>, Error<'a>>,
    end_delimiter: impl Parser<'a, &'a str, (), Error<'a>>,
) -> impl Parser<'a, &'a str, Vec<NamespaceChild<'a>>, Error<'a>> {
    choice((
        dto::parser(config).map(|(c, v)| Some((NamespaceChild::Dto(c), v))),
        rpc::parser(config).map(|(c, v)| Some((NamespaceChild::Rpc(c), v))),
        en::parser().map(|(c, v)| Some((NamespaceChild::Enum(c), v))),
        namespace.map(|(c, v)| Some((NamespaceChild::Namespace(c), v))),
        impl_block.map(|c| Some((NamespaceChild::Namespace(c), Visibility::Public))),
        const_or_alias().map(|_| None),
    ))
    .recover_with(skip_then_retry_until(
        any().ignored(),
        end_delimiter.ignored(),
    ))
    .map(|opt| match opt {
        Some((child, visibility)) => visibility.filter_child(child, config),
        None => None,
    })
    .repeated()
    .collect::<Vec<_>>()
    .map(|v| v.into_iter().flatten().collect_vec())
    .then_ignore(comment::multi_comment())
}

pub fn const_or_alias<'a>() -> impl Parser<'a, &'a str, (), Error<'a>> {
    comment::multi_comment()
        .then(visibility::parser())
        .then(just("const").or(just("type")))
        .then(any().and_is(none_of(";")).repeated().slice())
        .then(just(';'))
        .padded()
        .ignored()
}

// Parses to a 'virtual' namespace that will be merged into the DTO with the same name.
pub fn impl_block<'a>(
    config: &'a Config,
    namespace: impl Parser<'a, &'a str, (Namespace<'a>, Visibility), Error<'a>> + 'a,
) -> impl Parser<'a, &'a str, Namespace<'a>, Error<'a>> {
    recursive(|nested| {
        comment::multi_comment()
            .padded()
            .ignore_then(keyword_ex("impl"))
            .ignore_then(text::whitespace().at_least(1))
            .ignore_then(text::ident())
            .then(
                children(config, namespace, nested, just('}').ignored())
                    .delimited_by(just('{').padded(), just('}').padded()),
            )
            .map(|(name, children)| Namespace::<'a> {
                name: Cow::Borrowed(name),
                children,
                attributes: Default::default(),
                is_virtual: true,
            })
            .boxed()
    })
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use chumsky::Parser;

    use crate::model::{attribute, Comment, NamespaceChild};
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
                attribute::User::new_flag("flag1"),
                attribute::User::new_flag("flag2"),
            ]
        );
        Ok(())
    }

    #[test]
    fn impl_block() -> Result<()> {
        let namespace = namespace::impl_block(&TEST_CONFIG, namespace::parser(&TEST_CONFIG))
            .parse(
                r#"
                    impl dto {
                        fn rpc() {}
                    }
                    "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert!(namespace.is_virtual);
        Ok(())
    }

    mod const_or_alias {
        use crate::parser::rust::namespace;
        use crate::parser::test_util::wrap_test_err;
        use anyhow::Result;
        use chumsky::Parser;

        #[test]
        fn public_const() -> Result<()> {
            run_test("pub const ASDF: &[&str] = &[\"zz\", \"xx\"];")
        }

        #[test]
        fn private_const() -> Result<()> {
            run_test("const ASDF: &[&str] = &[\"zz\", \"xx\"];")
        }

        #[test]
        fn public_alias() -> Result<()> {
            run_test("pub type zzz = u128;")
        }

        #[test]
        fn private_alias() -> Result<()> {
            run_test("type zzz = u128;")
        }

        fn run_test(input: &'static str) -> Result<()> {
            namespace::const_or_alias()
                .parse(input)
                .into_result()
                .map_err(wrap_test_err)
        }
    }
}
