use crate::parser::is_static::is_static;
use crate::parser::visibility::Visibility;
use crate::parser::{attributes, comment, expr_block, ty, visibility};
use apyxl::model::{Attributes, Rpc};
use apyxl::parser::error::Error;
use apyxl::parser::{util, Config};
use chumsky::prelude::{any, choice, just};
use chumsky::{text, IterParser, Parser};
use itertools::Itertools;
use std::borrow::Cow;

enum Accessor {
    Get,
    Set,
}

pub fn parser(config: &Config) -> impl Parser<&str, Vec<(Rpc, Visibility)>, Error> {
    let semicolon = just(';').padded();
    let anything_until_semicolon = any()
        .and_is(just(';').not())
        .repeated()
        .slice()
        .then(semicolon);
    let initializer = just('=').padded().then(anything_until_semicolon);

    // x => value;
    let arrow_shorthand = just("=>").padded().then(anything_until_semicolon);

    let block_shorthand = expr_block::parser().padded();

    let accessor = |keyword| {
        visibility::parser(Visibility::Public)
            .then_ignore(util::keyword_ex(keyword))
            .then_ignore(choice((
                // get;
                semicolon.ignored(),
                // get => value;
                arrow_shorthand.ignored(),
                // get { value; }
                block_shorthand.clone().ignored(),
            )))
    };
    let accessor_block = choice((
        accessor("get").map(|visibility| (Accessor::Get, visibility)),
        accessor("set").map(|visibility| (Accessor::Set, visibility)),
    ))
    .repeated()
    .at_most(2)
    .collect::<Vec<_>>()
    .delimited_by(just('{').padded(), just('}').padded())
    .then_ignore(initializer.or_not());

    let field = ty::parser(config)
        .then_ignore(text::whitespace().at_least(1))
        .then(text::ident());

    comment::multi()
        .then(attributes::attributes().padded())
        .then(visibility::parser(Visibility::Private))
        .then(is_static())
        .then(field)
        .then(choice((
            // Public vis = use field visibility.
            arrow_shorthand.map(|_| vec![(Accessor::Get, Visibility::Public)]),
            accessor_block,
        )))
        .map(
            |(
                ((((comments, user), visibility), is_static), (return_ty, field_name)),
                accessors,
            )| {
                accessors
                    .into_iter()
                    .map(|(accessor, accessor_visibility)| {
                        let name = match accessor {
                            Accessor::Get => format!("get_{}", field_name),
                            Accessor::Set => format!("set_{}", field_name),
                        };

                        let rpc = Rpc {
                            name: Cow::Owned(name),
                            params: vec![],
                            return_type: Some(return_ty.clone()),
                            attributes: Attributes {
                                comments: comments.clone(),
                                user: user.clone(),
                                ..Default::default()
                            },
                            is_static,
                        };

                        let visibility = match (accessor_visibility, visibility) {
                            (Visibility::Public, field_visibility) => field_visibility,
                            (_, Visibility::Private) => Visibility::Private,
                            (Visibility::Private, _) => Visibility::Private,
                            // Note: some of these cases aren't valid C#, so we don't worry too much.
                            // e.g. internal x { get; protected set; } doesn't compile.
                            (accessor_visibility, _) => accessor_visibility,
                        };

                        (rpc, visibility)
                    })
                    .collect_vec()
            },
        )
}

#[cfg(test)]
mod tests {
    use crate::parser::property::{parser, Accessor};
    use crate::parser::visibility::Visibility;
    use anyhow::Result;
    use apyxl::model::attributes::User;
    use apyxl::model::{Comment, Rpc, Semantics, Type, TypeRef};
    use apyxl::parser::test_util::wrap_test_err;
    use apyxl::test_util::executor::TEST_CONFIG;
    use chumsky::Parser;

    #[test]
    fn type_parsed() -> Result<()> {
        let input = r#"
        string prop => 0;
        "#;
        check_property(input, Type::String, "prop", false, &[Accessor::Get])
    }

    #[test]
    fn shorthand_arrow() -> Result<()> {
        let input = r#"
        int prop => 0;
        "#;
        check_property(input, Type::I32, "prop", false, &[Accessor::Get])
    }

    #[test]
    fn block_get_no_body() -> Result<()> {
        let input = r#"
        int prop { get; }
        "#;
        check_property(input, Type::I32, "prop", false, &[Accessor::Get])
    }

    #[test]
    fn block_set_no_body() -> Result<()> {
        let input = r#"
        int prop { set; }
        "#;
        check_property(input, Type::I32, "prop", false, &[Accessor::Set])
    }

    #[test]
    fn block_both_no_body() -> Result<()> {
        let input = r#"
        int prop { get; set; }
        "#;
        check_property(
            input,
            Type::I32,
            "prop",
            false,
            &[Accessor::Get, Accessor::Set],
        )
    }

    #[test]
    fn block_get_arrow() -> Result<()> {
        let input = r#"
        int prop { get => 0; }
        "#;
        check_property(input, Type::I32, "prop", false, &[Accessor::Get])
    }

    #[test]
    fn block_set_arrow() -> Result<()> {
        let input = r#"
        int prop { set => 0; }
        "#;
        check_property(input, Type::I32, "prop", false, &[Accessor::Set])
    }

    #[test]
    fn block_both_arrow() -> Result<()> {
        let input = r#"
        int prop { get => 0; set => x = value; }
        "#;
        check_property(
            input,
            Type::I32,
            "prop",
            false,
            &[Accessor::Get, Accessor::Set],
        )
    }

    #[test]
    fn block_get_block() -> Result<()> {
        let input = r#"
        int prop { get { return 0; } }
        "#;
        check_property(input, Type::I32, "prop", false, &[Accessor::Get])
    }

    #[test]
    fn block_set_block() -> Result<()> {
        let input = r#"
        int prop { set { x = value; } }
        "#;
        check_property(input, Type::I32, "prop", false, &[Accessor::Set])
    }

    #[test]
    fn block_both_block() -> Result<()> {
        let input = r#"
        int prop { get { return 0; } set { x = value; } }
        "#;
        check_property(
            input,
            Type::I32,
            "prop",
            false,
            &[Accessor::Get, Accessor::Set],
        )
    }

    #[test]
    fn block_with_initializer() -> Result<()> {
        let input = r#"
        int prop { get; set; } = 12345;
        "#;
        check_property(
            input,
            Type::I32,
            "prop",
            false,
            &[Accessor::Get, Accessor::Set],
        )
    }

    #[test]
    fn public_accessor_uses_field_visibility() -> Result<()> {
        check_visibility("private int prop { public get; }", Visibility::Private)?;
        check_visibility("protected int prop { public get; }", Visibility::Protected)?;
        check_visibility("internal int prop { public get; }", Visibility::Internal)?;
        check_visibility("public int prop { public get; }", Visibility::Public)?;
        Ok(())
    }

    #[test]
    fn private_field_results_in_private() -> Result<()> {
        check_visibility("private int prop { public get; }", Visibility::Private)?;
        check_visibility("private int prop { protected get; }", Visibility::Private)?;
        check_visibility("private int prop { internal get; }", Visibility::Private)?;
        check_visibility("private int prop { private get; }", Visibility::Private)?;
        Ok(())
    }

    #[test]
    fn private_accessor_results_in_private() -> Result<()> {
        check_visibility("public int prop { private get; }", Visibility::Private)?;
        check_visibility("protected int prop { private get; }", Visibility::Private)?;
        check_visibility("internal int prop { private get; }", Visibility::Private)?;
        check_visibility("private int prop { private get; }", Visibility::Private)?;
        Ok(())
    }

    #[test]
    fn default_accessor_visibility_public() -> Result<()> {
        check_visibility("public int prop { get; }", Visibility::Public)?;
        Ok(())
    }

    #[test]
    fn default_property_visibility_private() -> Result<()> {
        check_visibility("int prop { public get; }", Visibility::Private)?;
        Ok(())
    }

    #[test]
    fn attributes_cloned_to_all_accessors() -> Result<()> {
        let input = r#"
        // prop comments
        [prop_attr]
        int prop { get; set; }
        "#;

        let property = parse_property(input)?;
        assert_eq!(property.len(), 2);

        let (get_rpc, _) = &property[0];
        let (set_rpc, _) = &property[1];

        assert_eq!(
            get_rpc.attributes.comments,
            vec![Comment::unowned(&["prop comments"])],
            "comments copied to get"
        );
        assert_eq!(
            set_rpc.attributes.comments,
            vec![Comment::unowned(&["prop comments"])],
            "comments copied to set"
        );

        assert_eq!(
            get_rpc.attributes.user,
            vec![User::new_flag("prop_attr")],
            "attrs copied to get"
        );
        assert_eq!(
            set_rpc.attributes.user,
            vec![User::new_flag("prop_attr")],
            "attrs copied to set"
        );
        Ok(())
    }

    #[test]
    fn static_prop() -> Result<()> {
        let input = r#"
        static int prop => 0;
        "#;
        check_property(input, Type::I32, "prop", true, &[Accessor::Get])
    }

    #[test]
    fn fails_on_normal_field() -> Result<()> {
        let input = r#"
        int prop;
        "#;
        let result = parser(&TEST_CONFIG).parse(input).into_result();
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn fails_on_normal_field_with_initializer() -> Result<()> {
        let input = r#"
        int prop = 0;
        "#;
        let result = parser(&TEST_CONFIG).parse(input).into_result();
        assert!(result.is_err());
        Ok(())
    }

    fn check_property(
        input: &'static str,
        ty: Type,
        name: &str,
        is_static: bool,
        accessors: &[Accessor],
    ) -> Result<()> {
        let property = parse_property(input)?;
        assert_eq!(property.len(), accessors.len());

        for (i, accessor) in accessors.iter().enumerate() {
            let (rpc, _) = &property[i];
            match accessor {
                Accessor::Get => assert_getter(rpc, ty.clone(), name, is_static),
                Accessor::Set => assert_setter(rpc, ty.clone(), name, is_static),
            }
        }
        Ok(())
    }

    fn assert_getter(rpc: &Rpc, ty: Type, name: &str, is_static: bool) {
        assert_accessor("get_", rpc, ty, name, is_static);
    }

    fn assert_setter(rpc: &Rpc, ty: Type, name: &str, is_static: bool) {
        assert_accessor("set_", rpc, ty, name, is_static);
    }

    fn assert_accessor(prefix: &str, rpc: &Rpc, ty: Type, name: &str, is_static: bool) {
        assert_eq!(rpc.name, format!("{}{}", prefix, name));
        assert_eq!(rpc.return_type, Some(TypeRef::new(ty, Semantics::Value)));
        assert_eq!(rpc.is_static, is_static);
        assert!(rpc.params.is_empty());
    }

    fn check_visibility(input: &'static str, expected: Visibility) -> Result<()> {
        let property = parse_property(input)?;
        assert!(!property.is_empty());
        let (_, actual) = property[0];
        assert_eq!(actual, expected);
        Ok(())
    }

    fn parse_property(input: &'static str) -> Result<Vec<(Rpc<'static>, Visibility)>> {
        let prop = parser(&TEST_CONFIG)
            .parse(input)
            .into_result()
            .map_err(wrap_test_err)?;
        Ok(prop)
    }
}
