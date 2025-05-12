use std::borrow::Cow;

use chumsky::prelude::*;
use itertools::Itertools;

use crate::parser::error::Error;
use crate::parser::util::keyword_ex;
use crate::parser::visibility::Visibility;
use crate::parser::{attributes, comment, dto, en, rpc, visibility};
use apyxl::model::{Attributes, Namespace, NamespaceChild};
use apyxl::parser::Config;

pub fn parser(config: &Config) -> impl Parser<&str, Namespace, Error> {
    recursive(|nested| {
        let prefix = keyword_ex("namespace").then(text::whitespace().at_least(1));
        let name = text::ident().separated_by(just(".")).collect::<Vec<_>>();
        let body = children(config, nested.clone(), just('}').ignored())
            .delimited_by(just('{').padded(), just('}').padded());
        comment::multi()
            .then(attributes::attributes().padded())
            .then_ignore(prefix)
            .then(name)
            .then(body)
            .map(|(((comments, user), name), children)| {
                Namespace {
                    // todo this needs to return nested set of namespaces, not a namespace w/ name glommed together...
                    name: Cow::Owned(name.join(".")),
                    children,
                    attributes: Attributes {
                        comments,
                        user,
                        ..Default::default()
                    },
                    is_virtual: false,
                }
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
        // todo
        rpc::parser(config).map(|(c, v)| Some((NamespaceChild::Rpc(c), v))),
        en::parser().map(|(c, v)| Some((NamespaceChild::Enum(c), v))),
        // ty_alias::parser(config).map(|(c, v)| Some((NamespaceChild::TypeAlias(c), v))),
        namespace.map(|c| Some((NamespaceChild::Namespace(c), Visibility::Public))),
        // const_var().map(|_| None),
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

pub fn const_var<'a>() -> impl Parser<'a, &'a str, (), Error<'a>> {
    comment::multi()
        .then(visibility::parser())
        .then(just("const"))
        .then(any().and_is(none_of(";")).repeated().slice())
        .then(just(';'))
        .padded()
        .ignored()
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
    fn with_dto() -> Result<()> {
        let namespace = namespace::parser(&TEST_CONFIG)
            .parse(
                r#"
            namespace ns {
                public struct DtoName {}
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

    mod const_var {
        use crate::parser::namespace;
        use apyxl::parser::test_util::wrap_test_err;
        use anyhow::Result;
        use chumsky::Parser;

        #[test]
        fn public_const() -> Result<()> {
            run_test("public const string ASDF = \"blah\";")
        }

        #[test]
        fn public_static() -> Result<()> {
            run_test("public static string ASDF = \"blah\";")
        }

        #[test]
        fn private_const() -> Result<()> {
            run_test("private const ASDF = \"blah\";")
        }

        #[test]
        fn private_static() -> Result<()> {
            run_test("private static string ASDF = \"blah\";")
        }

        fn run_test(input: &'static str) -> Result<()> {
            namespace::const_var()
                .parse(input)
                .into_result()
                .map_err(wrap_test_err)
        }
    }
}
