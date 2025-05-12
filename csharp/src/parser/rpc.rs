use crate::parser::visibility::Visibility;
use crate::parser::{attributes, comment, expr_block, ty, visibility};
use apyxl::model::{Attributes, Field, Rpc};
use apyxl::parser::Config;
use apyxl::parser::error::Error;
use chumsky::prelude::*;

pub fn parser(config: &Config) -> impl Parser<&str, (Rpc, Visibility), Error> {
    let return_type = choice((just("void").map(|_| None), ty::parser(config).map(Some)))
        .then_ignore(text::whitespace().at_least(1));
    let name = text::ident();
    let params = params(config).delimited_by(
        just('(').padded(),
        just(')').padded().recover_with(skip_then_retry_until(
            none_of(")").ignored(),
            just(')').ignored(),
        )),
    );
    comment::multi()
        .then(attributes::attributes().padded())
        .then(visibility::parser())
        .then(return_type)
        .then(name)
        .then(params)
        .then_ignore(expr_block::parser().padded())
        .map(
            |(((((comments, user), visibility), return_type), name), params)| {
                (
                    Rpc {
                        name,
                        params,
                        return_type,
                        attributes: Attributes {
                            comments,
                            user,
                            ..Default::default()
                        },
                    },
                    visibility,
                )
            },
        )
}

fn param(config: &Config) -> impl Parser<&str, Field, Error> {
    let field = ty::parser(config)
        .then_ignore(text::whitespace().at_least(1))
        .then(text::ident());
    comment::multi()
        .then(attributes::attributes().padded())
        .then(field)
        .map(|((comments, user), (ty, name))| Field {
            name,
            ty,
            attributes: Attributes {
                comments,
                user,
                ..Default::default()
            },
        })
}

fn params(config: &Config) -> impl Parser<&str, Vec<Field>, Error> {
    param(config)
        .separated_by(just(',').padded())
        .allow_trailing()
        .collect::<Vec<_>>()
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use chumsky::Parser;

    use crate::parser::rpc;
    use crate::parser::visibility::Visibility;
    use apyxl::model::{Comment, attributes};
    use apyxl::parser::test_util::wrap_test_err;
    use apyxl::test_util::executor::TEST_CONFIG;

    #[test]
    fn empty_fn() -> Result<()> {
        let (rpc, _) = rpc::parser(&TEST_CONFIG)
            .parse(
                r#"
            void rpc_name() {}
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(rpc.name, "rpc_name");
        assert!(rpc.params.is_empty());
        assert!(rpc.return_type.is_none());
        Ok(())
    }

    #[test]
    fn public() -> Result<()> {
        let (rpc, visibility) = rpc::parser(&TEST_CONFIG)
            .parse(
                r#"
            public void rpc_name() {}
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(rpc.name, "rpc_name");
        assert!(rpc.params.is_empty());
        assert!(rpc.return_type.is_none());
        assert_eq!(visibility, Visibility::Public);
        Ok(())
    }

    #[test]
    fn private() -> Result<()> {
        let (rpc, visibility) = rpc::parser(&TEST_CONFIG)
            .parse(
                r#"
            void rpc_name() {}
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(rpc.name, "rpc_name");
        assert_eq!(visibility, Visibility::Private);
        Ok(())
    }

    #[test]
    fn comment() -> Result<()> {
        let (rpc, _) = rpc::parser(&TEST_CONFIG)
            .parse(
                r#"
            // multi
            // line
            // comment
            void rpc() {}
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(
            rpc.attributes.comments,
            vec![Comment::unowned(&["multi", "line", "comment"])]
        );
        Ok(())
    }

    #[test]
    fn single_param() -> Result<()> {
        let (rpc, _) = rpc::parser(&TEST_CONFIG)
            .parse(
                r#"
            void rpc_name(ParamType0 param0) {}
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(rpc.params.len(), 1);
        assert_eq!(rpc.params[0].name, "param0");
        assert_eq!(
            rpc.params[0]
                .ty
                .value
                .api()
                .unwrap()
                .component_names()
                .last(),
            Some("ParamType0")
        );
        Ok(())
    }

    #[test]
    fn multiple_params() -> Result<()> {
        let (rpc, _) = rpc::parser(&TEST_CONFIG)
            .parse(
                r#"
            void rpc_name(ParamType0 param0, ParamType1 param1, ParamType2 param2) {}
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(rpc.params.len(), 3);
        assert_eq!(rpc.params[0].name, "param0");
        assert_eq!(
            rpc.params[0]
                .ty
                .value
                .api()
                .unwrap()
                .component_names()
                .last(),
            Some("ParamType0")
        );
        assert_eq!(rpc.params[1].name, "param1");
        assert_eq!(
            rpc.params[1]
                .ty
                .value
                .api()
                .unwrap()
                .component_names()
                .last(),
            Some("ParamType1")
        );
        assert_eq!(rpc.params[2].name, "param2");
        assert_eq!(
            rpc.params[2]
                .ty
                .value
                .api()
                .unwrap()
                .component_names()
                .last(),
            Some("ParamType2")
        );
        Ok(())
    }

    #[test]
    fn multiple_params_with_comments() -> Result<()> {
        let (rpc, _) = rpc::parser(&TEST_CONFIG)
            .parse(
                r#"
            void rpc_name(
                // multi
                // line
                ParamType0 param0, /* comment */ ParamType1 param1,
                // multi
                // line
                // comment
                ParamType2 param2
            ) {}
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(rpc.params.len(), 3);
        assert_eq!(
            rpc.params[0].attributes.comments,
            vec![Comment::unowned(&["multi", "line"])]
        );
        assert_eq!(
            rpc.params[1].attributes.comments,
            vec![Comment::unowned(&["comment"])]
        );
        assert_eq!(
            rpc.params[2].attributes.comments,
            vec![Comment::unowned(&["multi", "line", "comment"])]
        );
        Ok(())
    }

    #[test]
    fn multiple_params_weird_spacing_trailing_comma() -> Result<()> {
        let (rpc, _) = rpc::parser(&TEST_CONFIG)
            .parse(
                r#"
            void rpc_name(ParamType0 param0      , ParamType1
            param1     , ParamType2    param2

                ) {}
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(rpc.params.len(), 3);
        assert_eq!(rpc.params[0].name, "param0");
        assert_eq!(
            rpc.params[0]
                .ty
                .value
                .api()
                .unwrap()
                .component_names()
                .last(),
            Some("ParamType0")
        );
        assert_eq!(rpc.params[1].name, "param1");
        assert_eq!(
            rpc.params[1]
                .ty
                .value
                .api()
                .unwrap()
                .component_names()
                .last(),
            Some("ParamType1")
        );
        assert_eq!(rpc.params[2].name, "param2");
        assert_eq!(
            rpc.params[2]
                .ty
                .value
                .api()
                .unwrap()
                .component_names()
                .last(),
            Some("ParamType2")
        );
        Ok(())
    }

    #[test]
    fn return_type() -> Result<()> {
        let (rpc, _) = rpc::parser(&TEST_CONFIG)
            .parse(
                r#"
            Asdfg rpc_name() {}
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(
            rpc.return_type
                .as_ref()
                .map(|x| x.value.api().unwrap().component_names().last()),
            Some(Some("Asdfg"))
        );
        Ok(())
    }

    #[test]
    fn return_type_weird_spacing() -> Result<()> {
        let (rpc, _) = rpc::parser(&TEST_CONFIG)
            .parse(
                r#"
            Asdfg       rpc_name() {}
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(
            rpc.return_type
                .as_ref()
                .map(|x| x.value.api().unwrap().component_names().last()),
            Some(Some("Asdfg"))
        );
        Ok(())
    }

    #[test]
    fn attributes() -> Result<()> {
        let (rpc, _) = rpc::parser(&TEST_CONFIG)
            .parse(
                r#"
                [flag1, flag2]
                void rpc() {}
                "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(
            rpc.attributes.user,
            vec![
                attributes::User::new_flag("flag1"),
                attributes::User::new_flag("flag2"),
            ]
        );
        Ok(())
    }
}
