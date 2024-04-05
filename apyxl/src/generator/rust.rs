use std::path::Path;

use anyhow::Result;
use itertools::Itertools;

use crate::generator::{util, Generator};
use crate::model::{attributes, Chunk, Comment};
use crate::output::{Indented, Output};
use crate::rust_util;
use crate::view::{
    Attributes, Dto, EntityId, Enum, EnumValue, Field, InnerType, Model, Namespace, Rpc, SubView,
    Type, TypeAlias,
};

#[derive(Debug, Default)]
pub struct Rust {}

const INDENT: &str = "    "; // 4 spaces.

impl Generator for Rust {
    fn generate(&mut self, model: Model, output: &mut dyn Output) -> Result<()> {
        let mut o = Indented::new(output, INDENT);

        // Write combined API w/out chunks.
        write_namespace_contents(model.api(), &mut o)?;

        // Write chunked API.
        for result in model.api_chunked_iter() {
            let (chunk, sub_view) = result?;
            o.write_chunk(chunk)?;
            write_dependencies(&model, chunk, &sub_view, &mut o)?;
            write_namespace_contents(sub_view.namespace(), &mut o)?;
        }

        Ok(())
    }
}

fn write_dependencies(
    model: &Model,
    chunk: &Chunk,
    sub_view: &SubView,
    o: &mut dyn Output,
) -> Result<()> {
    let mut deps = util::collect_chunk_dependencies(
        &model.api(),
        &sub_view.root_id(),
        sub_view.namespace(),
        model.dependencies(),
    );
    // Don't import self.
    deps.retain(|path| path != chunk.relative_file_path.as_ref().unwrap());
    write_imports(&deps, o)
}

fn write_imports<P: AsRef<Path>>(chunk_relative_paths: &[P], o: &mut dyn Output) -> Result<()> {
    //
    // This generator uses fully-qualified types, which in rust means imports aren't necessary,
    // but it writes what it _would_ import in a comment.
    //
    let ids = chunk_relative_paths
        .iter()
        .map(|p| rust_util::path_to_entity_id(p.as_ref()))
        .filter(|id| !id.is_empty())
        .sorted()
        .dedup();
    for id in ids {
        o.write_str("// use crate::")?;
        for component in id.component_names() {
            o.write_str(&component)?;
            o.write_str("::")?;
        }
        o.write_str("*;")?;
        o.newline()?;
    }
    Ok(())
}

fn write_namespace(namespace: Namespace, o: &mut Indented) -> Result<()> {
    write_attributes(&namespace.attributes(), o)?;

    o.write_str("pub mod ")?;
    o.write_str(&namespace.name())?;

    if namespace.is_empty() {
        o.write(';')?;
    } else {
        o.write(' ')?;
        write_block_start(o)?;
        write_namespace_contents(namespace, o)?;
        write_block_end(o)?;
    }
    Ok(())
}

fn write_namespace_contents(namespace: Namespace, o: &mut Indented) -> Result<()> {
    for alias in namespace.ty_aliases() {
        write_alias(alias, o)?;
        o.newline()?;
    }

    for rpc in namespace.rpcs() {
        write_rpc(rpc, o)?;
        o.newline()?;
    }

    for en in namespace.enums() {
        write_enum(en, o)?;
        o.newline()?;
    }

    for dto in namespace.dtos() {
        write_dto(dto, o)?;
        o.newline()?;
    }

    for nested_ns in namespace.namespaces() {
        write_namespace(nested_ns, o)?;
        o.newline()?;
    }

    Ok(())
}

fn write_alias(alias: TypeAlias, o: &mut Indented) -> Result<()> {
    write_attributes(&alias.attributes(), o)?;

    o.write_str("pub type ")?;
    o.write_str(&alias.name())?;
    o.write_str(" = ")?;
    write_type(alias.target_ty(), o)?;
    o.write(';')?;
    o.newline()?;

    Ok(())
}

fn write_dto(dto: Dto, o: &mut Indented) -> Result<()> {
    write_attributes(&dto.attributes(), o)?;

    write_dto_start(dto, o)?;

    for field in dto.fields() {
        write_field(field, o)?;
        o.newline()?;
    }

    write_block_end(o)
}

fn write_rpc(rpc: Rpc, o: &mut Indented) -> Result<()> {
    write_attributes(&rpc.attributes(), o)?;

    o.write_str("pub fn ")?;
    o.write_str(&rpc.name())?;

    o.write('(')?;
    o.indent(1);
    for field in rpc.params() {
        o.newline()?;
        write_field(field, o)?;
    }
    o.indent(-1);

    if rpc.params().count() > 0 {
        o.newline()?;
    }

    o.write(')')?;

    if let Some(return_type) = rpc.return_type() {
        o.write_str(" -> ")?;
        write_type(return_type, o)?;
    }

    o.write_str(" {}")?;
    o.newline()
}

fn write_enum(en: Enum, o: &mut Indented) -> Result<()> {
    write_attributes(&en.attributes(), o)?;

    o.write_str("pub enum ")?;
    o.write_str(&en.name())?;
    o.write(' ')?;
    write_block_start(o)?;

    for value in en.values() {
        write_enum_value(value, o)?;
        o.newline()?;
    }

    write_block_end(o)
}

fn write_enum_value(value: EnumValue, o: &mut dyn Output) -> Result<()> {
    write_attributes(&value.attributes(), o)?;

    o.write_str(&value.name())?;
    o.write_str(" = ")?;
    o.write_str(&value.number().to_string())?;
    o.write(',')
}

fn write_dto_start(dto: Dto, o: &mut Indented) -> Result<()> {
    o.write_str("pub struct ")?;
    o.write_str(&dto.name())?;
    o.write(' ')?;
    write_block_start(o)
}

fn write_block_start(o: &mut Indented) -> Result<()> {
    o.write_str("{")?;
    o.indent(1);
    o.newline()
}

fn write_block_end(o: &mut Indented) -> Result<()> {
    o.indent(-1);
    o.write_str("}")?;
    o.newline()
}

fn write_field(field: Field, o: &mut dyn Output) -> Result<()> {
    write_param(field, o)?;
    o.write(',')
}

fn write_param(field: Field, o: &mut dyn Output) -> Result<()> {
    write_attributes(&field.attributes(), o)?;

    o.write_str(&field.name())?;
    o.write_str(": ")?;
    write_type(field.ty(), o)
}

fn write_attributes(attributes: &Attributes, o: &mut dyn Output) -> Result<()> {
    write_comments(&attributes.comments(), o)?;
    write_user_attributes(attributes.user(), o)?;
    Ok(())
}

fn write_comments(comments: &[Comment], o: &mut dyn Output) -> Result<()> {
    util::write_joined(comments, "\n", o, |comment, o| {
        for line in comment.lines() {
            o.write_str("// ")?;
            o.write_str(line)?;
            o.newline()?;
        }
        Ok(())
    })?;
    Ok(())
}

fn write_user_attributes(user_attributes: &[attributes::User], o: &mut dyn Output) -> Result<()> {
    if user_attributes.is_empty() {
        return Ok(());
    }
    o.write_str("#[")?;
    util::write_joined(user_attributes, ", ", o, |attr, o| {
        write_user_attribute(attr.name, &attr.data, o)
    })?;
    o.write(']')?;
    o.newline()?;
    Ok(())
}

fn write_user_attribute(
    name: &str,
    data: &[attributes::UserData],
    o: &mut dyn Output,
) -> Result<()> {
    o.write_str(name)?;
    if data.is_empty() {
        return Ok(());
    }
    o.write('(')?;
    util::write_joined(data, ", ", o, |data, o| {
        match data.key {
            None => {}
            Some(key) => {
                o.write_str(key)?;
                o.write_str(" = ")?;
            }
        }
        o.write_str(data.value)
    })?;
    o.write(')')?;
    Ok(())
}

fn write_type(ty: Type, o: &mut dyn Output) -> Result<()> {
    write_inner_type(ty.inner(), o)
}

fn write_inner_type(ty: InnerType, o: &mut dyn Output) -> Result<()> {
    match ty {
        InnerType::Bool => o.write_str("bool"),
        InnerType::U8 => o.write_str("u8"),
        InnerType::U16 => o.write_str("u16"),
        InnerType::U32 => o.write_str("u32"),
        InnerType::U64 => o.write_str("u64"),
        InnerType::U128 => o.write_str("u128"),
        InnerType::USIZE => o.write_str("usize"),
        InnerType::I8 => o.write_str("i8"),
        InnerType::I16 => o.write_str("i16"),
        InnerType::I32 => o.write_str("i32"),
        InnerType::I64 => o.write_str("i64"),
        InnerType::I128 => o.write_str("i128"),
        InnerType::F8 => o.write_str("f8"),
        InnerType::F16 => o.write_str("f16"),
        InnerType::F32 => o.write_str("f32"),
        InnerType::F64 => o.write_str("f64"),
        InnerType::F128 => o.write_str("f128"),
        InnerType::String => o.write_str("String"),
        InnerType::Bytes => o.write_str("Vec<u8>"),
        // For the sake of example, just write the user type name.
        InnerType::User(s) => o.write_str(s),
        InnerType::Api(id) => write_entity_id(id, o),
        InnerType::Array(ty) => write_vec(*ty, o),
        InnerType::Map { key, value } => write_map(*key, *value, o),
        InnerType::Optional(ty) => write_option(*ty, o),
    }
}

fn write_entity_id(entity_id: EntityId, o: &mut dyn Output) -> Result<()> {
    // Fully qualify everything by crate.
    o.write_str("crate::")?;
    util::write_joined_str(
        &entity_id.path().iter().map(|s| s.as_ref()).collect_vec(),
        "::",
        o,
    )
}

fn write_vec(ty: InnerType, o: &mut dyn Output) -> Result<()> {
    o.write_str("Vec<")?;
    write_inner_type(ty, o)?;
    o.write('>')
}

fn write_map(key: InnerType, value: InnerType, o: &mut dyn Output) -> Result<()> {
    o.write_str("HashMap<")?;
    write_inner_type(key, o)?;
    o.write_str(", ")?;
    write_inner_type(value, o)?;
    o.write('>')
}

fn write_option(ty: InnerType, o: &mut dyn Output) -> Result<()> {
    o.write_str("Option<")?;
    write_inner_type(ty, o)?;
    o.write('>')
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::generator::rust::{
        write_dto, write_entity_id, write_enum, write_field, write_rpc, INDENT,
    };
    use crate::generator::util::tests::{assert_e2e, assert_output, assert_output_slice, indent};
    use crate::generator::Rust;
    use crate::model::{attributes, Attributes};
    use crate::output::Indented;
    use crate::view::Transforms;
    use crate::{model, view};

    #[test]
    fn full_generation() -> Result<()> {
        assert_e2e::<Rust>(
            r#"
pub enum EnumName {
    One = 1,
    Two,
    Three = 99,
}

pub fn rpc_name(
    dto: DtoName,
    dto2: ns0::DtoName,
) -> DtoName {}

pub struct DtoName {
    i: i32,
}

pub mod ns0 {
    pub struct DtoName {
        i: i32,
    }
}
"#,
            r#"pub fn rpc_name(
    dto: crate::DtoName,
    dto2: crate::ns0::DtoName,
) -> crate::DtoName {}

pub enum EnumName {
    One = 1,
    Two = 2,
    Three = 99,
}

pub struct DtoName {
    i: i32,
}

pub mod ns0 {
    pub struct DtoName {
        i: i32,
    }

}

"#,
        )
    }

    #[test]
    fn dto() -> Result<()> {
        assert_output_slice(
            |o| {
                write_dto(
                    view::Dto::new(
                        &model::Dto {
                            name: "DtoName",
                            fields: vec![
                                model::Field {
                                    name: "field0",
                                    ty: model::Type::new_api("Type0")?,
                                    attributes: test_attributes(),
                                },
                                model::Field {
                                    name: "field1",
                                    ty: model::Type::new_api("Type1")?,
                                    attributes: test_attributes(),
                                },
                            ],
                            attributes: test_attributes(),
                            namespace: None,
                        },
                        &Transforms::default(),
                    ),
                    &mut Indented::new(o, INDENT),
                )
            },
            &[
                expected_attribute_str(),
                "pub struct DtoName {",
                &indent("    ", expected_attribute_str()),
                "    field0: crate::Type0,",
                &indent("    ", expected_attribute_str()),
                "    field1: crate::Type1,",
                "}\n",
            ],
        )
    }

    #[test]
    fn rpc() -> Result<()> {
        assert_output_slice(
            |o| {
                write_rpc(
                    view::Rpc::new(
                        &model::Rpc {
                            name: "rpc_name",
                            params: vec![
                                model::Field {
                                    name: "param0",
                                    ty: model::Type::new_api("Type0")?,
                                    attributes: test_attributes(),
                                },
                                model::Field {
                                    name: "param1",
                                    ty: model::Type::new_api("Type1")?,
                                    attributes: test_attributes(),
                                },
                            ],
                            return_type: None,
                            attributes: test_attributes(),
                        },
                        &Transforms::default(),
                    ),
                    &mut Indented::new(o, INDENT),
                )
            },
            &[
                expected_attribute_str(),
                "pub fn rpc_name(",
                &indent("    ", expected_attribute_str()),
                "    param0: crate::Type0,",
                &indent("    ", expected_attribute_str()),
                "    param1: crate::Type1,",
                ") {}\n",
            ],
        )
    }

    #[test]
    fn rpc_with_return() -> Result<()> {
        assert_output(
            |o| {
                write_rpc(
                    view::Rpc::new(
                        &model::Rpc {
                            name: "rpc_name",
                            params: vec![],
                            return_type: Some(model::Type::new_api("ReturnType")?),
                            attributes: Default::default(),
                        },
                        &Transforms::default(),
                    ),
                    &mut Indented::new(o, INDENT),
                )
            },
            "pub fn rpc_name() -> crate::ReturnType {}\n",
        )
    }

    #[test]
    fn field() -> Result<()> {
        assert_output_slice(
            |o| {
                write_field(
                    view::Field::new(
                        &model::Field {
                            name: "asdf",
                            ty: model::Type::new_api("Type")?,
                            attributes: test_attributes(),
                        },
                        &vec![],
                        &vec![],
                        &vec![],
                    ),
                    o,
                )
            },
            &[expected_attribute_str(), "asdf: crate::Type,"],
        )
    }

    #[test]
    fn en() -> Result<()> {
        assert_output_slice(
            |o| {
                write_enum(
                    view::Enum::new(
                        &model::Enum {
                            name: "en",
                            values: vec![
                                model::EnumValue {
                                    name: "value0",
                                    number: 10,
                                    attributes: test_attributes(),
                                },
                                model::EnumValue {
                                    name: "value1",
                                    number: 20,
                                    attributes: test_attributes(),
                                },
                            ],
                            attributes: test_attributes(),
                        },
                        &Transforms::default(),
                    ),
                    &mut Indented::new(o, INDENT),
                )
            },
            &[
                expected_attribute_str(),
                "pub enum en {",
                &indent("    ", expected_attribute_str()),
                "    value0 = 10,",
                &indent("    ", expected_attribute_str()),
                "    value1 = 20,",
                "}\n",
            ],
        )
    }

    fn test_attributes<'a>() -> Attributes<'a> {
        Attributes {
            user: vec![
                attributes::User::new_flag("flag"),
                attributes::User::new(
                    "list",
                    vec![
                        attributes::UserData::new(None, "Abc"),
                        attributes::UserData::new(None, "Def"),
                    ],
                ),
                attributes::User::new(
                    "map",
                    vec![
                        attributes::UserData::new(Some("a"), "1"),
                        attributes::UserData::new(Some("b"), "2"),
                    ],
                ),
            ],
            ..Default::default()
        }
    }

    fn expected_attribute_str() -> &'static str {
        "#[flag, list(Abc, Def), map(a = 1, b = 2)]"
    }

    mod imports {
        use anyhow::Result;

        use crate::generator::rust::write_imports;
        use crate::generator::util::tests::assert_output;

        #[test]
        fn with_extension() -> Result<()> {
            assert_output(
                |o| write_imports(&["a/b/c.rs"], o),
                "// use crate::a::b::c::*;\n",
            )
        }

        #[test]
        fn without_extension() -> Result<()> {
            assert_output(
                |o| write_imports(&["a/b/c"], o),
                "// use crate::a::b::c::*;\n",
            )
        }

        #[test]
        fn mod_rs() -> Result<()> {
            assert_output(
                |o| write_imports(&["a/b/mod.rs"], o),
                "// use crate::a::b::*;\n",
            )
        }

        #[test]
        fn lib_rs() -> Result<()> {
            assert_output(
                |o| write_imports(&["a/b/lib.rs"], o),
                "// use crate::a::b::*;\n",
            )
        }

        #[test]
        fn no_duplicates() -> Result<()> {
            assert_output(
                |o| write_imports(&["a/b/c.rs", "a/b/c", "a/b/c/mod.rs"], o),
                "// use crate::a::b::c::*;\n",
            )
        }

        #[test]
        fn multiple() -> Result<()> {
            assert_output(
                |o| write_imports(&["a", "a/b", "a/b/c"], o),
                r#"// use crate::a::*;
// use crate::a::b::*;
// use crate::a::b::c::*;
"#,
            )
        }

        #[test]
        fn empty() -> Result<()> {
            assert_output(|o| write_imports(&["lib.rs"], o), "")
        }
    }

    mod ty {
        use anyhow::Result;

        use crate::generator::rust::write_type;
        use crate::generator::util::tests::assert_output;
        use crate::model;
        use crate::view::Type;

        macro_rules! test {
            ($name:ident, $expected:literal, $ty:expr) => {
                #[test]
                fn $name() -> Result<()> {
                    run_test($ty, $expected)
                }
            };
        }

        test!(bool, "bool", model::Type::Bool);
        test!(u8, "u8", model::Type::U8);
        test!(u16, "u16", model::Type::U16);
        test!(u32, "u32", model::Type::U32);
        test!(u64, "u64", model::Type::U64);
        test!(u128, "u128", model::Type::U128);
        test!(i8, "i8", model::Type::I8);
        test!(i16, "i16", model::Type::I16);
        test!(i32, "i32", model::Type::I32);
        test!(i64, "i64", model::Type::I64);
        test!(i128, "i128", model::Type::I128);
        test!(f8, "f8", model::Type::F8);
        test!(f16, "f16", model::Type::F16);
        test!(f32, "f32", model::Type::F32);
        test!(f64, "f64", model::Type::F64);
        test!(f128, "f128", model::Type::F128);
        test!(string, "String", model::Type::String);
        test!(bytes, "Vec<u8>", model::Type::Bytes);
        test!(
            entity_id,
            "crate::a::b::c",
            model::Type::Api(model::EntityId::try_from("a.b.c").unwrap())
        );
        test!(
            vec,
            "Vec<String>",
            model::Type::new_array(model::Type::String)
        );
        test!(
            option,
            "Option<String>",
            model::Type::new_optional(model::Type::String)
        );
        test!(
            map,
            "HashMap<String, i32>",
            model::Type::new_map(model::Type::String, model::Type::I32)
        );

        fn run_test(ty: model::Type, expected: &str) -> Result<()> {
            assert_output(|o| write_type(Type::new(&ty, &vec![]), o), expected)
        }
    }

    #[test]
    fn entity_id() -> Result<()> {
        let entity_id = model::EntityId::try_from("a.b.c")?;
        assert_output(
            |o| write_entity_id(view::EntityId::new(&entity_id, &vec![]), o),
            "crate::a::b::c",
        )
    }
}
