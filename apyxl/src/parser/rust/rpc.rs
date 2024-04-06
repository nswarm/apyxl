use crate::model::{Attributes, Field, Rpc, Semantics, Type, TypeRef};
use crate::parser::error::Error;
use crate::parser::rust::visibility::Visibility;
use crate::parser::rust::{attributes, expr_block, ty, visibility};
use crate::parser::{comment, util, Config};
use chumsky::prelude::*;

pub fn parser(config: &Config) -> impl Parser<&str, (Rpc, Visibility), Error> {
    let prefix = util::keyword_ex("fn").then(text::whitespace().at_least(1));
    let name = text::ident();
    let params = params(config).delimited_by(
        just('(').padded(),
        just(')').padded().recover_with(skip_then_retry_until(
            none_of(")").ignored(),
            just(')').ignored(),
        )),
    );
    let return_type = just("->").ignore_then(ty::parser(config).padded());
    comment::multi_comment()
        .then(attributes::attributes().padded())
        .then(visibility::parser())
        .then_ignore(prefix)
        .then(name)
        .then(params)
        .then(return_type.or_not())
        .then_ignore(expr_block::parser().padded())
        .map(
            |(((((comments, user), visibility), name), params), return_type)| {
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
    let self_param = choice((
        just("self").map(|ty| {
            (
                "self",
                TypeRef::new(Type::User(ty.to_string()), Semantics::Value),
            )
        }),
        just("&self").map(|ty| {
            (
                "self",
                TypeRef::new(Type::User(ty.to_string()), Semantics::Ref),
            )
        }),
        just("&mut self").map(|ty| {
            (
                "self",
                TypeRef::new(Type::User(ty.to_string()), Semantics::Mut),
            )
        }),
    ));
    let field = text::ident()
        .then_ignore(just(':').padded())
        .then(ty::parser(config));
    comment::multi_comment()
        .then(attributes::attributes().padded())
        .then(field.or(self_param))
        .map(|((comments, user), (name, ty))| Field {
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

    use crate::model::{attributes, Comment, Semantics, Type, TypeRef};
    use crate::parser::rust::rpc;
    use crate::parser::rust::visibility::Visibility;
    use crate::parser::test_util::wrap_test_err;
    use crate::test_util::executor::TEST_CONFIG;

    #[test]
    fn empty_fn() -> Result<()> {
        let (rpc, _) = rpc::parser(&TEST_CONFIG)
            .parse(
                r#"
            fn rpc_name() {}
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
    fn self_param() -> Result<()> {
        let (rpc, _) = rpc::parser(&TEST_CONFIG)
            .parse(
                r#"
            fn rpc_name(self) {}
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(rpc.params.len(), 1);
        assert_eq!(
            rpc.params[0].ty,
            TypeRef::new(Type::User("self".to_string()), Semantics::Value)
        );
        assert_eq!(rpc.params[0].name, "self");
        Ok(())
    }

    #[test]
    fn self_ref_param() -> Result<()> {
        let (rpc, _) = rpc::parser(&TEST_CONFIG)
            .parse(
                r#"
            fn rpc_name(&self) {}
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(rpc.params.len(), 1);
        assert_eq!(
            rpc.params[0].ty,
            TypeRef::new(Type::User("&self".to_string()), Semantics::Ref)
        );
        assert_eq!(rpc.params[0].name, "self");
        Ok(())
    }

    #[test]
    fn self_mut_param() -> Result<()> {
        let (rpc, _) = rpc::parser(&TEST_CONFIG)
            .parse(
                r#"
            fn rpc_name(&mut self) {}
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(rpc.params.len(), 1);
        assert_eq!(
            rpc.params[0].ty,
            TypeRef::new(Type::User("&mut self".to_string()), Semantics::Mut)
        );
        assert_eq!(rpc.params[0].name, "self");
        Ok(())
    }

    #[test]
    fn public() -> Result<()> {
        let (rpc, visibility) = rpc::parser(&TEST_CONFIG)
            .parse(
                r#"
            pub fn rpc_name() {}
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
            fn rpc_name() {}
            "#,
            )
            .into_result()
            .map_err(wrap_test_err)?;
        assert_eq!(rpc.name, "rpc_name");
        assert_eq!(visibility, Visibility::Private);
        Ok(())
    }

    #[test]
    fn fn_keyword_smushed() {
        let rpc = rpc::parser(&TEST_CONFIG)
            .parse(
                r#"
            pubfn rpc_name() {}
            "#,
            )
            .into_result();
        assert!(rpc.is_err());
    }

    #[test]
    fn comment() -> Result<()> {
        let (rpc, _) = rpc::parser(&TEST_CONFIG)
            .parse(
                r#"
            // multi
            // line
            // comment
            fn rpc() {}
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
            fn rpc_name(param0: ParamType0) {}
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
            fn rpc_name(param0: ParamType0, param1: ParamType1, param2: ParamType2) {}
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
            fn rpc_name(
                // multi
                // line
                param0: ParamType0, /* comment */ param1: ParamType1,
                // multi
                // line
                // comment
                param2: ParamType2
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
            fn rpc_name(param0: ParamType0      , param1
            :    ParamType1     , param2 :ParamType2
                ,
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
            fn rpc_name() -> Asdfg {}
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
            fn rpc_name()           ->Asdfg{}
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
                #[flag1, flag2]
                fn rpc() {}
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
