use std::path::{Path, PathBuf};

use anyhow::Result;
use itertools::Itertools;

use crate::generator::Generator;
use crate::model::{Chunk, Dependencies, EntityType};
use crate::output::{Indented, Output};
use crate::view::{
    Dto, EntityId, Enum, EnumValue, Field, InnerType, Model, Namespace, Rpc, SubView, Type,
};
use crate::{model, rust_util};

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
    let mut deps = collect_chunk_dependencies(
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

fn write_dto(dto: Dto, o: &mut Indented) -> Result<()> {
    write_dto_start(dto, o)?;

    for field in dto.fields() {
        write_field(field, o)?;
        o.newline()?;
    }

    write_block_end(o)
}

fn write_rpc(rpc: Rpc, o: &mut Indented) -> Result<()> {
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
    o.write_str("enum ")?;
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
    o.write_str(&value.name())?;
    o.write_str(" = ")?;
    o.write_str(&value.number().to_string())?;
    o.write(',')
}

fn write_dto_start(dto: Dto, o: &mut Indented) -> Result<()> {
    o.write_str("struct ")?;
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
    o.write_str(&field.name())?;
    o.write_str(": ")?;
    write_type(field.ty(), o)
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
    write_joined(
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

/// Writes the `components` joined with `separator` without unnecessary allocations.
fn write_joined(components: &[&str], separator: &str, o: &mut dyn Output) -> Result<()> {
    let len = components.len();
    for (i, component) in components.iter().enumerate() {
        o.write_str(component)?;
        if i < len - 1 {
            o.write_str(separator)?;
        }
    }
    Ok(())
}

/// Collects relative paths for every chunk referenced by any child (recursively) within `dependent_ns`.
fn collect_chunk_dependencies<'v, 'a>(
    root: &'v Namespace<'v, 'a>,
    dependent_id: &model::EntityId,
    dependent_ns: Namespace<'v, 'a>,
    dependencies: &'v Dependencies,
) -> Vec<PathBuf> {
    collect_dependencies_recursively(dependent_id, dependent_ns, dependencies)
        .iter()
        .flat_map(|id| match root.find_child(&id) {
            None => vec![],
            Some(child) => match child.attributes().chunk() {
                None => vec![],
                Some(attr) => attr.relative_file_paths.clone(),
            },
        })
        .dedup()
        .collect_vec()
}

/// Collects all [model::EntityId]s that `dependent` [Namespace] depends on by recursing the
/// hierarchy and collecting all dependents of each [NamespaceChild].
fn collect_dependencies_recursively<'a>(
    dependent_id: &model::EntityId,
    dependent_ns: Namespace,
    dependencies: &'a Dependencies,
) -> Vec<&'a model::EntityId> {
    let child_dependencies = dependent_ns
        .children()
        .map(|child| {
            // unwrap ok: we're iterating over known children.
            dependent_id
                .child(child.entity_type(), child.name())
                .unwrap()
        })
        .flat_map(|id| dependencies.get_for(&id));
    dependent_ns
        .namespaces()
        .flat_map(|ns| {
            // unwrap ok: we're iterating over known children.
            collect_dependencies_recursively(
                &dependent_id
                    .child(EntityType::Namespace, ns.name())
                    .unwrap(),
                ns,
                dependencies,
            )
        })
        .chain(child_dependencies)
        .collect_vec()
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::generator::rust::{
        write_dto, write_entity_id, write_enum, write_field, write_rpc, INDENT,
    };
    use crate::generator::Rust;
    use crate::output::Indented;
    use crate::test_util::executor::TestExecutor;
    use crate::view::Transforms;
    use crate::{model, output, view, Generator};

    #[test]
    fn full_generation() -> Result<()> {
        let data = r#"
pub fn rpc_name(
    dto: DtoName,
    dto2: ns0::DtoName,
) -> DtoName {}

struct DtoName {
    i: i32,
}

pub mod ns0 {
    struct DtoName {
        i: i32,
    }
}
"#;
        let expected = r#"pub fn rpc_name(
    dto: crate::DtoName,
    dto2: crate::ns0::DtoName,
) -> crate::DtoName {}

struct DtoName {
    i: i32,
}

pub mod ns0 {
    struct DtoName {
        i: i32,
    }

}

"#;
        let mut exe = TestExecutor::new(data);
        let model = exe.model();
        let view = model.view();
        assert_output(move |o| Rust::default().generate(view, o), expected)
    }

    #[test]
    fn dto() -> Result<()> {
        assert_output(
            |o| {
                write_dto(
                    view::Dto::new(
                        &model::Dto {
                            name: "DtoName",
                            fields: vec![
                                model::Field {
                                    name: "field0",
                                    ty: model::Type::new_api("Type0")?,
                                    attributes: Default::default(),
                                },
                                model::Field {
                                    name: "field1",
                                    ty: model::Type::new_api("Type1")?,
                                    attributes: Default::default(),
                                },
                            ],
                            attributes: Default::default(),
                        },
                        &Transforms::default(),
                    ),
                    &mut Indented::new(o, INDENT),
                )
            },
            r#"struct DtoName {
    field0: crate::Type0,
    field1: crate::Type1,
}
"#,
        )
    }

    #[test]
    fn rpc() -> Result<()> {
        assert_output(
            |o| {
                write_rpc(
                    view::Rpc::new(
                        &model::Rpc {
                            name: "rpc_name",
                            params: vec![
                                model::Field {
                                    name: "param0",
                                    ty: model::Type::new_api("Type0")?,
                                    attributes: Default::default(),
                                },
                                model::Field {
                                    name: "param1",
                                    ty: model::Type::new_api("Type1")?,
                                    attributes: Default::default(),
                                },
                            ],
                            return_type: None,
                            attributes: Default::default(),
                        },
                        &Transforms::default(),
                    ),
                    &mut Indented::new(o, INDENT),
                )
            },
            r#"pub fn rpc_name(
    param0: crate::Type0,
    param1: crate::Type1,
) {}
"#,
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
        assert_output(
            |o| {
                write_field(
                    view::Field::new(
                        &model::Field {
                            name: "asdf",
                            ty: model::Type::new_api("Type")?,
                            attributes: Default::default(),
                        },
                        &vec![],
                        &vec![],
                        &vec![],
                    ),
                    o,
                )
            },
            "asdf: crate::Type,",
        )
    }

    #[test]
    fn en() -> Result<()> {
        assert_output(
            |o| {
                write_enum(
                    view::Enum::new(
                        &model::Enum {
                            name: "en",
                            values: vec![
                                model::EnumValue {
                                    name: "value0",
                                    number: 10,
                                    attributes: Default::default(),
                                },
                                model::EnumValue {
                                    name: "value1",
                                    number: 20,
                                    attributes: Default::default(),
                                },
                            ],
                            attributes: Default::default(),
                        },
                        &Transforms::default(),
                    ),
                    &mut Indented::new(o, INDENT),
                )
            },
            r#"enum en {
    value0 = 10,
    value1 = 20,
}
"#,
        )
    }

    mod imports {
        use crate::generator::rust::tests::assert_output;
        use crate::generator::rust::write_imports;
        use anyhow::Result;

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

        use crate::generator::rust::tests::assert_output;
        use crate::generator::rust::write_type;
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

    fn assert_output<F: FnOnce(&mut output::Buffer) -> Result<()>>(
        write: F,
        expected: &str,
    ) -> Result<()> {
        let mut output = output::Buffer::default();
        write(&mut output)?;
        assert_eq!(&output.to_string(), expected);
        Ok(())
    }
}
