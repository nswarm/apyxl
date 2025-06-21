// use std::borrow::Cow;
// use crate::parser::is_static::is_static;
// use crate::parser::visibility::Visibility;
// use crate::parser::{attributes, comment, ty, visibility};
// use apyxl::model::{Attributes, Field, Rpc};
// use apyxl::parser::error::Error;
// use apyxl::parser::{util, Config};
// use chumsky::prelude::{any, just};
// use chumsky::{text, Parser};
//
// pub fn parser(config: &Config) -> impl Parser<&str, Vec<(Rpc, Visibility)>, Error> {
//     let end = just(';');
//     let initializer = just('=')
//         .padded()
//         .then(any().and_is(end.not()).repeated().slice());
//     let field = ty::parser(config)
//         .then_ignore(text::whitespace().at_least(1))
//         .then(text::ident())
//         .then_ignore(initializer.or_not())
//         .then_ignore(end.padded());
//     comment::multi()
//         .then(attributes::attributes().padded())
//         .then(visibility::parser(Visibility::Private))
//         .then(is_static())
//         .then_ignore(util::keyword_ex("event"))
//         .then(field)
//         .map(
//             |((((comments, user), visibility), is_static), (ty, name))| {
//                 let mut rpcs = Vec::new();
//                 // todo also support `delegate`... somehow...
//                 // todo function type?
//                 //      ..hmm do I need a function type? wait I have a function type, it's called rpc. uhhh
//                 //      ....do I need.... types able to be of type rpc?.....?
//                 //      would that solve this?
//                 //      hmm
//                 //      public delegate int SomeFunc(string x, int y); // this is a type... maybe it's an alias? i.e. the type is int(string,int), alias is SomeFunc
//                 //      okayyyyyy then...
//                 //      fields could just be of that type. i.e. public event SomeFunc OnSomeFunc;
//                 //      ahhhh this makes sense now. very good.
//
//                 rpcs.push((Rpc {
//                     name: Cow::Owned(format!("add_{}_listener", name)),
//                     params: vec![],
//                     return_type: None,
//                     attributes: Default::default(),
//                     is_static,
//                 }, visibility))
//
//                 rpcs
//
//                 // add_<event_name>_listener` and `remove_<event_name>_listener
//
//             },
//         )
// }
//
// #[cfg(test)]
// mod tests {
//     use crate::parser::property::{parser, Accessor};
//     use crate::parser::visibility::Visibility;
//     use anyhow::Result;
//     use apyxl::model::attributes::User;
//     use apyxl::model::{Comment, Rpc, Semantics, Type, TypeRef};
//     use apyxl::parser::test_util::wrap_test_err;
//     use apyxl::test_util::executor::TEST_CONFIG;
//     use chumsky::Parser;
//
//     #[test]
//     fn type_parsed() -> Result<()> {
//         let input = r#"
//         string prop => 0;
//         "#;
//         check_property(input, Type::String, "prop", false, &[Accessor::Get])
//     }
//
//     #[test]
//     fn shorthand_arrow() -> Result<()> {
//         let input = r#"
//         int prop => 0;
//         "#;
//         check_property(input, Type::I32, "prop", false, &[Accessor::Get])
//     }
//
//     #[test]
//     fn block_get_no_body() -> Result<()> {
//         let input = r#"
//         int prop { get; }
//         "#;
//         check_property(input, Type::I32, "prop", false, &[Accessor::Get])
//     }
//
//     #[test]
//     fn block_set_no_body() -> Result<()> {
//         let input = r#"
//         int prop { set; }
//         "#;
//         check_property(input, Type::I32, "prop", false, &[Accessor::Set])
//     }
//
//     #[test]
//     fn block_both_no_body() -> Result<()> {
//         let input = r#"
//         int prop { get; set; }
//         "#;
//         check_property(
//             input,
//             Type::I32,
//             "prop",
//             false,
//             &[Accessor::Get, Accessor::Set],
//         )
//     }
//
//     #[test]
//     fn block_get_arrow() -> Result<()> {
//         let input = r#"
//         int prop { get => 0; }
//         "#;
//         check_property(input, Type::I32, "prop", false, &[Accessor::Get])
//     }
//
//     #[test]
//     fn block_set_arrow() -> Result<()> {
//         let input = r#"
//         int prop { set => 0; }
//         "#;
//         check_property(input, Type::I32, "prop", false, &[Accessor::Set])
//     }
//
//     #[test]
//     fn block_both_arrow() -> Result<()> {
//         let input = r#"
//         int prop { get => 0; set => x = value; }
//         "#;
//         check_property(
//             input,
//             Type::I32,
//             "prop",
//             false,
//             &[Accessor::Get, Accessor::Set],
//         )
//     }
//
//     #[test]
//     fn block_get_block() -> Result<()> {
//         let input = r#"
//         int prop { get { return 0; } }
//         "#;
//         check_property(input, Type::I32, "prop", false, &[Accessor::Get])
//     }
//
//     #[test]
//     fn block_set_block() -> Result<()> {
//         let input = r#"
//         int prop { set { x = value; } }
//         "#;
//         check_property(input, Type::I32, "prop", false, &[Accessor::Set])
//     }
//
//     #[test]
//     fn block_both_block() -> Result<()> {
//         let input = r#"
//         int prop { get { return 0; } set { x = value; } }
//         "#;
//         check_property(
//             input,
//             Type::I32,
//             "prop",
//             false,
//             &[Accessor::Get, Accessor::Set],
//         )
//     }
//
//     #[test]
//     fn block_with_initializer() -> Result<()> {
//         let input = r#"
//         int prop { get; set; } = 12345;
//         "#;
//         check_property(
//             input,
//             Type::I32,
//             "prop",
//             false,
//             &[Accessor::Get, Accessor::Set],
//         )
//     }
//
//     #[test]
//     fn public_accessor_uses_field_visibility() -> Result<()> {
//         check_visibility("private int prop { public get; }", Visibility::Private)?;
//         check_visibility("protected int prop { public get; }", Visibility::Protected)?;
//         check_visibility("internal int prop { public get; }", Visibility::Internal)?;
//         check_visibility("public int prop { public get; }", Visibility::Public)?;
//         Ok(())
//     }
//
//     #[test]
//     fn private_field_results_in_private() -> Result<()> {
//         check_visibility("private int prop { public get; }", Visibility::Private)?;
//         check_visibility("private int prop { protected get; }", Visibility::Private)?;
//         check_visibility("private int prop { internal get; }", Visibility::Private)?;
//         check_visibility("private int prop { private get; }", Visibility::Private)?;
//         Ok(())
//     }
//
//     #[test]
//     fn private_accessor_results_in_private() -> Result<()> {
//         check_visibility("public int prop { private get; }", Visibility::Private)?;
//         check_visibility("protected int prop { private get; }", Visibility::Private)?;
//         check_visibility("internal int prop { private get; }", Visibility::Private)?;
//         check_visibility("private int prop { private get; }", Visibility::Private)?;
//         Ok(())
//     }
//
//     #[test]
//     fn default_accessor_visibility_public() -> Result<()> {
//         check_visibility("public int prop { get; }", Visibility::Public)?;
//         Ok(())
//     }
//
//     #[test]
//     fn default_property_visibility_private() -> Result<()> {
//         check_visibility("int prop { public get; }", Visibility::Private)?;
//         Ok(())
//     }
//
//     #[test]
//     fn attributes_cloned_to_all_accessors() -> Result<()> {
//         let input = r#"
//         // prop comments
//         [prop_attr]
//         int prop { get; set; }
//         "#;
//
//         let property = parse_property(input)?;
//         assert_eq!(property.len(), 2);
//
//         let (get_rpc, _) = &property[0];
//         let (set_rpc, _) = &property[1];
//
//         assert_eq!(
//             get_rpc.attributes.comments,
//             vec![Comment::unowned(&["prop comments"])],
//             "comments copied to get"
//         );
//         assert_eq!(
//             set_rpc.attributes.comments,
//             vec![Comment::unowned(&["prop comments"])],
//             "comments copied to set"
//         );
//
//         assert_eq!(
//             get_rpc.attributes.user,
//             vec![User::new_flag("prop_attr")],
//             "attrs copied to get"
//         );
//         assert_eq!(
//             set_rpc.attributes.user,
//             vec![User::new_flag("prop_attr")],
//             "attrs copied to set"
//         );
//         Ok(())
//     }
//
//     #[test]
//     fn static_prop() -> Result<()> {
//         let input = r#"
//         static int prop => 0;
//         "#;
//         check_property(input, Type::I32, "prop", true, &[Accessor::Get])
//     }
//
//     #[test]
//     fn fails_on_normal_field() -> Result<()> {
//         let input = r#"
//         int prop;
//         "#;
//         let result = parser(&TEST_CONFIG).parse(input).into_result();
//         assert!(result.is_err());
//         Ok(())
//     }
//
//     #[test]
//     fn fails_on_normal_field_with_initializer() -> Result<()> {
//         let input = r#"
//         int prop = 0;
//         "#;
//         let result = parser(&TEST_CONFIG).parse(input).into_result();
//         assert!(result.is_err());
//         Ok(())
//     }
//
//     fn check_property(
//         input: &'static str,
//         ty: Type,
//         name: &str,
//         is_static: bool,
//         accessors: &[Accessor],
//     ) -> Result<()> {
//         let property = parse_property(input)?;
//         assert_eq!(property.len(), accessors.len());
//
//         for (i, accessor) in accessors.iter().enumerate() {
//             let (rpc, _) = &property[i];
//             match accessor {
//                 Accessor::Get => assert_getter(rpc, ty.clone(), name, is_static),
//                 Accessor::Set => assert_setter(rpc, ty.clone(), name, is_static),
//             }
//         }
//         Ok(())
//     }
//
//     fn assert_getter(rpc: &Rpc, ty: Type, name: &str, is_static: bool) {
//         assert_accessor("get_", rpc, ty, name, is_static);
//     }
//
//     fn assert_setter(rpc: &Rpc, ty: Type, name: &str, is_static: bool) {
//         assert_accessor("set_", rpc, ty, name, is_static);
//     }
//
//     fn assert_accessor(prefix: &str, rpc: &Rpc, ty: Type, name: &str, is_static: bool) {
//         assert_eq!(rpc.name, format!("{}{}", prefix, name));
//         assert_eq!(rpc.return_type, Some(TypeRef::new(ty, Semantics::Value)));
//         assert_eq!(rpc.is_static, is_static);
//         assert!(rpc.params.is_empty());
//     }
//
//     fn check_visibility(input: &'static str, expected: Visibility) -> Result<()> {
//         let property = parse_property(input)?;
//         assert!(!property.is_empty());
//         let (_, actual) = property[0];
//         assert_eq!(actual, expected);
//         Ok(())
//     }
//
//     fn parse_property(input: &'static str) -> Result<Vec<(Rpc<'static>, Visibility)>> {
//         let prop = parser(&TEST_CONFIG)
//             .parse(input)
//             .into_result()
//             .map_err(wrap_test_err)?;
//         Ok(prop)
//     }
// }
