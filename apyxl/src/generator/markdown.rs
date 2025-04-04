use anyhow::Result;
use itertools::Itertools;

use crate::generator::{util, Generator};
use crate::model::{attributes, Comment, Semantics};
use crate::output::Output;
use crate::view::{
    Attributes, Dto, EntityId, Enum, EnumValue, Field, Model, Namespace, Rpc, Type,
    TypeAlias, TypeRef,
};

#[derive(Debug, Default)]
pub struct Markdown {}

impl Generator for Markdown {
    fn generate(&mut self, model: Model, output: &mut dyn Output) -> Result<()> {
        let mut o = output;
        let mut header = Header::new();

        // Write combined API w/out chunks.
        write_namespace_contents(model.api(), o, &mut header)?;

        // Write chunked API.
        // for result in model.api_chunked_iter() {
        //     let (chunk, sub_view) = result?;
        //     o.write_chunk(chunk)?;
        //     write_namespace_contents(sub_view.namespace(), o, &mut header)?;
        // }

        Ok(())
    }
}

static MAX_PREFIX: &str = "###### "; // The space is load-bearing.
/// Track the current header depth (H1-H6).
struct Header {
    depth: u8,
}
impl Header {
    pub fn new() -> Self {
        Self {
            depth: 1,
        }
    }

    pub fn write(&self, name: &str, o: &mut dyn Output) -> Result<()> {
        o.write(self.prefix())?;
        o.write(name)?;
        o.newline()?;
        Ok(())
    }

    pub fn prefix(&self) -> &'static str {
        let max = MAX_PREFIX.len();
        let depth = self.depth as usize;
        let start = max - depth - 1;
        &MAX_PREFIX[start..]
    }

    pub fn bigger(&mut self) {
        self.depth = std::cmp::max(self.depth - 1, 1);
    }

    pub fn smaller(&mut self) {
        self.depth = std::cmp::min(self.depth + 1, (MAX_PREFIX.len() - 1) as u8);
    }
}

fn write_namespace(namespace: Namespace, o: &mut dyn Output, header: &mut Header) -> Result<()> {
    write_attributes(&namespace.attributes(), o)?;

    let mut title = String::new();
    title.push_str("Namespace ");
    title.push_str(&namespace.name());
    header.write(&title, o)?;

    header.smaller();
    if namespace.is_empty() {
        o.write("This namespace has no contents.")?;
        o.newline()?;
        o.newline()?;
    } else {
        write_namespace_contents(namespace, o, header)?;
    }
    header.bigger();

    Ok(())
}

fn write_namespace_contents(namespace: Namespace, o: &mut dyn Output, header: &mut Header) -> Result<()> {
    if namespace.ty_aliases().count() > 0 {
        header.write("Type aliases", o)?;
        header.smaller();
        for alias in namespace.ty_aliases() {
            write_alias(alias, o, header)?;
            o.newline()?;
        }
        header.bigger();
        o.newline()?;
    }

    if namespace.rpcs().count() > 0 {
        header.write("RPCs", o)?;
        header.smaller();
        for rpc in namespace.rpcs() {
            write_rpc(rpc, o, header)?;
            o.newline()?;
        }
        header.bigger();
        o.newline()?;
    }

    if namespace.enums().count() > 0 {
        header.write("Enums", o)?;
        header.smaller();
        for en in namespace.enums() {
            write_enum(en, o, header)?;
            o.newline()?;
        }
        header.bigger();
        o.newline()?;
    }

    if namespace.dtos().count() > 0 {
        header.write("DTOs", o)?;
        header.smaller();
        for dto in namespace.dtos() {
            write_dto(dto, o, header)?;
            o.newline()?;
        }
        header.bigger();
        o.newline()?;
    }

    // todo handle differently
    for nested_ns in namespace.namespaces() {
        write_namespace(nested_ns, o, header)?;
        o.newline()?;
    }
    header.bigger();

    Ok(())
}

fn write_alias(alias: TypeAlias, o: &mut dyn Output, header: &mut Header) -> Result<()> {
    // todo markdown
    write_attributes(&alias.attributes(), o)?;

    o.write("pub type ")?;
    o.write(&alias.name())?;
    o.write(" = ")?;
    write_type(alias.target_ty(), o)?;
    o.write_char(';')?;
    o.newline()?;

    Ok(())
}

fn write_dto(dto: Dto, o: &mut dyn Output, header: &mut Header) -> Result<()> {
    header.write(&dto.name(), o)?;
    header.smaller();

    write_attributes(&dto.attributes(), o)?;

    if dto.fields().count() > 0 {
        header.write("Fields", o)?;
        header.smaller();
        for field in dto.fields() {
            write_field(field, o, header)?;
            o.newline()?;
        }
        o.newline()?;
        header.bigger();
    }

    if let Some(ns) = dto.namespace() {
        if ns.rpcs().count() > 0 {
            header.write("RPCs", o)?;
            header.smaller();
            for rpc in ns.rpcs() {
                o.newline()?;
                write_rpc(rpc, o, header)?;
            }
            o.newline()?;
            header.bigger();
        }
    }
    o.newline()?;

    header.bigger();
    Ok(())
}

fn write_rpc(rpc: Rpc, o: &mut dyn Output, header: &mut Header) -> Result<()> {
    header.write(&rpc.name(), o)?;

    write_attributes(&rpc.attributes(), o)?;

    header.write("Parameters", o)?;
    header.smaller();
    for field in rpc.params() {
        o.newline()?;
        write_field(field, o, header)?;
    }
    header.bigger();

    if rpc.params().count() > 0 {
        o.newline()?;
    }

    if let Some(return_type) = rpc.return_type() {
        header.write("Returns", o)?;
        write_type(return_type, o)?;
    }
    o.newline()
}

fn write_enum(en: Enum, o: &mut dyn Output, header: &mut Header) -> Result<()> {
    // todo
    write_attributes(&en.attributes(), o)?;

    o.write("pub enum ")?;
    o.write(&en.name())?;
    o.write_char(' ')?;
    // write_block_start(o)?;

    for value in en.values() {
        write_enum_value(value, o)?;
        o.newline()?;
    }

    // write_block_end(o)
    Ok(())
}

fn write_enum_value(value: EnumValue, o: &mut dyn Output) -> Result<()> {
    // todo
    write_attributes(&value.attributes(), o)?;

    o.write(&value.name())?;
    o.write(" = ")?;
    o.write(&value.number().to_string())?;
    o.write_char(',')
}

fn write_field(field: Field, o: &mut dyn Output, header: &mut Header) -> Result<()> {
    write_param(field, o, header)
}

fn write_param(field: Field, o: &mut dyn Output, header: &mut Header) -> Result<()> {
    o.write("**")?;
    o.write("Name:")?;
    o.write("**")?;
    o.write_char(' ')?;
    o.write(&field.name())?;
    o.newline()?;
    o.newline()?;

    o.write("**")?;
    o.write("Type:")?;
    o.write("**")?;
    o.write_char(' ')?;
    write_type(field.ty(), o)?;
    o.newline()?;
    o.newline()?;

    if field.attributes().comments().len() > 0 {
        o.write("Description:")?;
        o.newline()?;
        o.newline()?;
        write_attributes(&field.attributes(), o)?;
    }

    Ok(())
}

fn write_attributes(attributes: &Attributes, o: &mut dyn Output) -> Result<()> {
    write_comments(&attributes.comments(), o)?;
    // write_user_attributes(&attributes.user(), o)?;
    Ok(())
}

fn write_comments(comments: &[Comment], o: &mut dyn Output) -> Result<()> {
    util::write_joined(comments, "\n", o, |comment, o| {
        for line in comment.lines() {
            o.write(line)?;
            o.newline()?;
            o.newline()?;
        }
        Ok(())
    })?;
    Ok(())
}

fn write_user_attributes(user_attributes: &[attributes::User], o: &mut dyn Output) -> Result<()> {
    // todo
    if user_attributes.is_empty() {
        return Ok(());
    }
    o.write("#[")?;
    util::write_joined(user_attributes, ", ", o, |attr, o| {
        write_user_attribute(attr.name.as_ref(), &attr.data, o)
    })?;
    o.write_char(']')?;
    o.newline()?;
    Ok(())
}

fn write_user_attribute(
    name: &str,
    data: &[attributes::UserData],
    o: &mut dyn Output,
) -> Result<()> {
    o.write(name)?;
    if data.is_empty() {
        return Ok(());
    }
    o.write_char('(')?;
    util::write_joined(data, ", ", o, |data, o| {
        match data.key {
            None => {}
            Some(key) => {
                o.write(key)?;
                o.write(" = ")?;
            }
        }
        o.write(data.value)
    })?;
    o.write_char(')')?;
    Ok(())
}

fn write_type(ty: TypeRef, o: &mut dyn Output) -> Result<()> {
    write_semantics(ty.semantics(), o)?;
    write_inner_type(ty, o)
}

fn write_inner_type(ty: TypeRef, o: &mut dyn Output) -> Result<()> {
    // todo
    match ty.value() {
        Type::Bool => o.write("bool"),
        Type::U8 => o.write("u8"),
        Type::U16 => o.write("u16"),
        Type::U32 => o.write("u32"),
        Type::U64 => o.write("u64"),
        Type::U128 => o.write("u128"),
        Type::USIZE => o.write("usize"),
        Type::I8 => o.write("i8"),
        Type::I16 => o.write("i16"),
        Type::I32 => o.write("i32"),
        Type::I64 => o.write("i64"),
        Type::I128 => o.write("i128"),
        Type::F8 => o.write("f8"),
        Type::F16 => o.write("f16"),
        Type::F32 => o.write("f32"),
        Type::F64 => o.write("f64"),
        Type::F128 => o.write("f128"),
        Type::StringView => o.write("&str"),
        Type::String => o.write("String"),
        Type::Bytes => o.write("Vec<u8>"),
        // For the sake of example, just write the user type name.
        Type::User(s) => o.write(s),
        Type::Api(id) => write_entity_id(id, o),
        Type::Array(array_ty) => write_vec(*array_ty, o),
        Type::Map { key, value } => write_map(*key, *value, o),
        Type::Optional(opt_ty) => write_option(*opt_ty, o),
    }
}

fn write_entity_id(entity_id: EntityId, o: &mut dyn Output) -> Result<()> {
    // Fully qualify everything by crate.
    o.write("crate::")?;
    util::write_joined_str(
        &entity_id.path().iter().map(|s| s.as_ref()).collect_vec(),
        "::",
        o,
    )
}

fn write_semantics(semantics: Semantics, o: &mut dyn Output) -> Result<()> {
    match semantics {
        Semantics::Value => Ok(()),
        Semantics::Ref => o.write_char('&'),
        Semantics::Mut => o.write("&mut "),
    }
}

fn write_vec(ty: TypeRef, o: &mut dyn Output) -> Result<()> {
    o.write("Vec<")?;
    write_type(ty, o)?;
    o.write_char('>')
}

fn write_map(key: TypeRef, value: TypeRef, o: &mut dyn Output) -> Result<()> {
    o.write("HashMap<")?;
    write_type(key, o)?;
    o.write(", ")?;
    write_type(value, o)?;
    o.write_char('>')
}

fn write_option(ty: TypeRef, o: &mut dyn Output) -> Result<()> {
    o.write("Option<")?;
    write_type(ty, o)?;
    o.write_char('>')
}

// todo
// #[cfg(test)]
// mod tests {
//     use anyhow::Result;
//
//     use crate::generator::rust::{
//         write_dto, write_entity_id, write_enum, write_field, write_rpc, INDENT,
//     };
//     use crate::generator::util::tests::{assert_e2e, assert_output, assert_output_slice, indent};
//     use crate::generator::Rust;
//     use crate::model::{attributes, Attributes, Semantics, Type};
//     use crate::output::Indented;
//     use crate::view::Transforms;
//     use crate::{model, view};
//
//     #[test]
//     fn full_generation() -> Result<()> {
//         assert_e2e::<Rust>(
//             r#"
// pub enum EnumName {
//     One = 1,
//     Two,
//     Three = 99,
// }
//
// pub fn rpc_name(
//     dto: DtoName,
//     dto2: ns0::DtoName,
// ) -> DtoName {}
//
// pub struct DtoName {
//     i: i32,
// }
//
// pub mod ns0 {
//     pub struct DtoName {
//         i: i32,
//     }
// }
// "#,
//             r#"pub fn rpc_name(
//     dto: crate::DtoName,
//     dto2: crate::ns0::DtoName,
// ) -> crate::DtoName {}
//
// pub enum EnumName {
//     One = 1,
//     Two = 2,
//     Three = 99,
// }
//
// pub struct DtoName {
//     i: i32,
// }
//
// pub mod ns0 {
//     pub struct DtoName {
//         i: i32,
//     }
//
// }
//
// "#,
//         )
//     }
//
//     #[test]
//     fn dto() -> Result<()> {
//         assert_output_slice(
//             |o| {
//                 write_dto(
//                     view::Dto::new(
//                         &model::Dto {
//                             name: "DtoName",
//                             fields: vec![
//                                 model::Field {
//                                     name: "field0",
//                                     ty: model::TypeRef::new_api("Type0", Semantics::Value)?,
//                                     attributes: test_attributes(),
//                                 },
//                                 model::Field {
//                                     name: "field1",
//                                     ty: model::TypeRef::new_api("Type1", Semantics::Ref)?,
//                                     attributes: test_attributes(),
//                                 },
//                                 model::Field {
//                                     name: "field2",
//                                     ty: model::TypeRef::new_api("Type2", Semantics::Mut)?,
//                                     attributes: test_attributes(),
//                                 },
//                             ],
//                             attributes: test_attributes(),
//                             namespace: None,
//                         },
//                         &Transforms::default(),
//                     ),
//                     &mut Indented::new(o, INDENT),
//                 )
//             },
//             &[
//                 expected_attribute_str(),
//                 "pub struct DtoName {",
//                 &indent("    ", expected_attribute_str()),
//                 "    field0: crate::Type0,",
//                 &indent("    ", expected_attribute_str()),
//                 "    field1: &crate::Type1,",
//                 &indent("    ", expected_attribute_str()),
//                 "    field2: &mut crate::Type2,",
//                 "}\n",
//             ],
//         )
//     }
//
//     #[test]
//     fn rpc() -> Result<()> {
//         assert_output_slice(
//             |o| {
//                 write_rpc(
//                     view::Rpc::new(
//                         &model::Rpc {
//                             name: "rpc_name",
//                             params: vec![
//                                 model::Field {
//                                     name: "param0",
//                                     ty: model::TypeRef::new_api("Type0", Semantics::Value)?,
//                                     attributes: test_attributes(),
//                                 },
//                                 model::Field {
//                                     name: "param1",
//                                     ty: model::TypeRef::new_api("Type1", Semantics::Ref)?,
//                                     attributes: test_attributes(),
//                                 },
//                             ],
//                             return_type: None,
//                             attributes: test_attributes(),
//                         },
//                         &Transforms::default(),
//                     ),
//                     &mut Indented::new(o, INDENT),
//                 )
//             },
//             &[
//                 expected_attribute_str(),
//                 "pub fn rpc_name(",
//                 &indent("    ", expected_attribute_str()),
//                 "    param0: crate::Type0,",
//                 &indent("    ", expected_attribute_str()),
//                 "    param1: &crate::Type1,",
//                 ") {}\n",
//             ],
//         )
//     }
//
//     #[test]
//     fn rpc_with_return() -> Result<()> {
//         assert_output(
//             |o| {
//                 write_rpc(
//                     view::Rpc::new(
//                         &model::Rpc {
//                             name: "rpc_name",
//                             params: vec![],
//                             return_type: Some(model::TypeRef::new_api(
//                                 "ReturnType",
//                                 Semantics::Ref,
//                             )?),
//                             attributes: Default::default(),
//                         },
//                         &Transforms::default(),
//                     ),
//                     &mut Indented::new(o, INDENT),
//                 )
//             },
//             "pub fn rpc_name() -> &crate::ReturnType {}\n",
//         )
//     }
//
//     #[test]
//     fn field() -> Result<()> {
//         assert_output_slice(
//             |o| {
//                 write_field(
//                     view::Field::new(
//                         &model::Field {
//                             name: "asdf",
//                             ty: model::TypeRef::new_api("Type", Semantics::Value)?,
//                             attributes: test_attributes(),
//                         },
//                         &vec![],
//                         &vec![],
//                         &vec![],
//                     ),
//                     o,
//                 )
//             },
//             &[expected_attribute_str(), "asdf: crate::Type,"],
//         )
//     }
//
//     #[test]
//     fn field_self() -> Result<()> {
//         assert_output_slice(
//             |o| {
//                 write_field(
//                     view::Field::new(
//                         &model::Field {
//                             name: "self",
//                             ty: model::TypeRef {
//                                 value: Type::User("&mut self".to_string()),
//                                 semantics: Semantics::Mut,
//                             },
//                             attributes: Attributes::default(),
//                         },
//                         &vec![],
//                         &vec![],
//                         &vec![],
//                     ),
//                     o,
//                 )
//             },
//             &["&mut self,"],
//         )
//     }
//
//     #[test]
//     fn en() -> Result<()> {
//         assert_output_slice(
//             |o| {
//                 write_enum(
//                     view::Enum::new(
//                         &model::Enum {
//                             name: "en",
//                             values: vec![
//                                 model::EnumValue {
//                                     name: "value0",
//                                     number: 10,
//                                     attributes: test_attributes(),
//                                 },
//                                 model::EnumValue {
//                                     name: "value1",
//                                     number: 20,
//                                     attributes: test_attributes(),
//                                 },
//                             ],
//                             attributes: test_attributes(),
//                         },
//                         &Transforms::default(),
//                     ),
//                     &mut Indented::new(o, INDENT),
//                 )
//             },
//             &[
//                 expected_attribute_str(),
//                 "pub enum en {",
//                 &indent("    ", expected_attribute_str()),
//                 "    value0 = 10,",
//                 &indent("    ", expected_attribute_str()),
//                 "    value1 = 20,",
//                 "}\n",
//             ],
//         )
//     }
//
//     fn test_attributes<'a>() -> Attributes<'a> {
//         Attributes {
//             user: vec![
//                 attributes::User::new_flag("flag"),
//                 attributes::User::new(
//                     "list",
//                     vec![
//                         attributes::UserData::new(None, "Abc"),
//                         attributes::UserData::new(None, "Def"),
//                     ],
//                 ),
//                 attributes::User::new(
//                     "map",
//                     vec![
//                         attributes::UserData::new(Some("a"), "1"),
//                         attributes::UserData::new(Some("b"), "2"),
//                     ],
//                 ),
//             ],
//             ..Default::default()
//         }
//     }
//
//     fn expected_attribute_str() -> &'static str {
//         "#[flag, list(Abc, Def), map(a = 1, b = 2)]"
//     }
//
//     mod imports {
//         use anyhow::Result;
//
//         use crate::generator::rust::write_imports;
//         use crate::generator::util::tests::assert_output;
//
//         #[test]
//         fn with_extension() -> Result<()> {
//             assert_output(
//                 |o| write_imports(&["a/b/c.rs"], o),
//                 "// use crate::a::b::c::*;\n",
//             )
//         }
//
//         #[test]
//         fn without_extension() -> Result<()> {
//             assert_output(
//                 |o| write_imports(&["a/b/c"], o),
//                 "// use crate::a::b::c::*;\n",
//             )
//         }
//
//         #[test]
//         fn mod_rs() -> Result<()> {
//             assert_output(
//                 |o| write_imports(&["a/b/mod.rs"], o),
//                 "// use crate::a::b::*;\n",
//             )
//         }
//
//         #[test]
//         fn lib_rs() -> Result<()> {
//             assert_output(
//                 |o| write_imports(&["a/b/lib.rs"], o),
//                 "// use crate::a::b::*;\n",
//             )
//         }
//
//         #[test]
//         fn no_duplicates() -> Result<()> {
//             assert_output(
//                 |o| write_imports(&["a/b/c.rs", "a/b/c", "a/b/c/mod.rs"], o),
//                 "// use crate::a::b::c::*;\n",
//             )
//         }
//
//         #[test]
//         fn multiple() -> Result<()> {
//             assert_output(
//                 |o| write_imports(&["a", "a/b", "a/b/c"], o),
//                 r#"// use crate::a::*;
// // use crate::a::b::*;
// // use crate::a::b::c::*;
// "#,
//             )
//         }
//
//         #[test]
//         fn empty() -> Result<()> {
//             assert_output(|o| write_imports(&["lib.rs"], o), "")
//         }
//     }
//
//     mod ty {
//         use anyhow::Result;
//
//         use crate::generator::rust::write_type;
//         use crate::generator::util::tests::assert_output;
//         use crate::model::{Semantics, Type, TypeRef};
//         use crate::view;
//
//         macro_rules! test {
//             ($name:ident, $expected:literal, $ty:expr) => {
//                 #[test]
//                 fn $name() -> Result<()> {
//                     run_test($ty, $expected)
//                 }
//             };
//         }
//
//         test!(bool, "bool", TypeRef::new(Type::Bool, Semantics::Value));
//         test!(u8, "u8", TypeRef::new(Type::U8, Semantics::Value));
//         test!(u16, "u16", TypeRef::new(Type::U16, Semantics::Value));
//         test!(u32, "u32", TypeRef::new(Type::U32, Semantics::Value));
//         test!(u64, "u64", TypeRef::new(Type::U64, Semantics::Value));
//         test!(u128, "u128", TypeRef::new(Type::U128, Semantics::Value));
//         test!(i8, "i8", TypeRef::new(Type::I8, Semantics::Value));
//         test!(i16, "i16", TypeRef::new(Type::I16, Semantics::Value));
//         test!(i32, "i32", TypeRef::new(Type::I32, Semantics::Value));
//         test!(i64, "i64", TypeRef::new(Type::I64, Semantics::Value));
//         test!(i128, "i128", TypeRef::new(Type::I128, Semantics::Value));
//         test!(f8, "f8", TypeRef::new(Type::F8, Semantics::Value));
//         test!(f16, "f16", TypeRef::new(Type::F16, Semantics::Value));
//         test!(f32, "f32", TypeRef::new(Type::F32, Semantics::Value));
//         test!(f64, "f64", TypeRef::new(Type::F64, Semantics::Value));
//         test!(f128, "f128", TypeRef::new(Type::F128, Semantics::Value));
//         test!(
//             string,
//             "String",
//             TypeRef::new(Type::String, Semantics::Value)
//         );
//         test!(
//             bytes,
//             "Vec<u8>",
//             TypeRef::new(Type::Bytes, Semantics::Value)
//         );
//         test!(
//             entity_id_value,
//             "crate::a::b::c",
//             TypeRef::new_api("a.b.c", Semantics::Value).unwrap()
//         );
//         test!(
//             entity_id_ref,
//             "&crate::a::b::c",
//             TypeRef::new_api("a.b.c", Semantics::Ref).unwrap()
//         );
//         test!(
//             entity_id_mut,
//             "&mut crate::a::b::c",
//             TypeRef::new_api("a.b.c", Semantics::Mut).unwrap()
//         );
//         test!(
//             vec,
//             "Vec<String>",
//             TypeRef::new_array(
//                 TypeRef::new(Type::String, Semantics::Value),
//                 Semantics::Value
//             )
//         );
//         test!(
//             option,
//             "Option<String>",
//             TypeRef::new_optional(
//                 TypeRef::new(Type::String, Semantics::Value),
//                 Semantics::Value
//             )
//         );
//         test!(
//             map,
//             "HashMap<String, i32>",
//             TypeRef::new_map(
//                 TypeRef::new(Type::String, Semantics::Value),
//                 TypeRef::new(Type::I32, Semantics::Value),
//                 Semantics::Value
//             )
//         );
//
//         fn run_test(ty: TypeRef, expected: &str) -> Result<()> {
//             assert_output(
//                 |o| write_type(view::TypeRef::new(&ty, &vec![]), o),
//                 expected,
//             )
//         }
//     }
//
//     #[test]
//     fn entity_id() -> Result<()> {
//         let entity_id = model::EntityId::try_from("a.b.c")?;
//         assert_output(
//             |o| write_entity_id(view::EntityId::new(&entity_id, &vec![]), o),
//             "crate::a::b::c",
//         )
//     }
// }
