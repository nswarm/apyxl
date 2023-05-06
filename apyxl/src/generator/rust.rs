use anyhow::{anyhow, Result};
use itertools::Itertools;

use crate::generator::Generator;
use crate::output::{Indented, Output};
use crate::view::{Dto, EntityId, Field, InnerType, Model, Namespace, Rpc, Type};

#[derive(Debug, Default)]
pub struct Rust {}

const INDENT: &str = "    ";

impl Generator for Rust {
    fn generate(&mut self, model: Model, output: &mut dyn Output) -> Result<()> {
        let mut o = Indented::new(output, INDENT);

        // Write combined API w/out chunks.
        write_namespace_contents(model.api(), &mut o)?;

        // Write chunked API.
        for result in model.api_chunked_iter() {
            let (chunk, sub_view) = result?;
            o.write_chunk(chunk)?;
            write_namespace_contents(sub_view.namespace(), &mut o)?;
        }

        Ok(())
    }
}

fn write_namespace(namespace: Namespace, o: &mut Indented) -> Result<()> {
    o.write_str("pub mod ")?;
    o.write_str(&namespace.name())?;
    o.write(' ')?;
    write_block_start(o)?;
    write_namespace_contents(namespace, o)?;
    write_block_end(o)
}

fn write_namespace_contents(namespace: Namespace, o: &mut Indented) -> Result<()> {
    for rpc in namespace.rpcs() {
        write_rpc(rpc, o)?;
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
    match ty.inner() {
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
        InnerType::User(s) => return Err(anyhow!("generator does not support user type '{}'", s)),
        InnerType::Api(id) => write_entity_id(id, o),
    }
}

fn write_entity_id(entity_id: EntityId, o: &mut dyn Output) -> Result<()> {
    write_joined(
        &entity_id.path().iter().map(|s| s.as_ref()).collect_vec(),
        "::",
        o,
    )
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

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::generator::rust::{write_dto, write_entity_id, write_field, write_rpc, INDENT};
    use crate::generator::Rust;
    use crate::output::Indented;
    use crate::test_util::executor::TestExecutor;
    use crate::view::Transforms;
    use crate::{model, output, view, Generator};

    #[test]
    fn full_generation() -> Result<()> {
        let expected = r#"pub fn rpc_name(
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
        let mut exe = TestExecutor::new(expected);
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
                                    ty: model::Type::new_api("Type0"),
                                    attributes: Default::default(),
                                },
                                model::Field {
                                    name: "field1",
                                    ty: model::Type::new_api("Type1"),
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
    field0: Type0,
    field1: Type1,
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
                                    ty: model::Type::new_api("Type0"),
                                    attributes: Default::default(),
                                },
                                model::Field {
                                    name: "param1",
                                    ty: model::Type::new_api("Type1"),
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
    param0: Type0,
    param1: Type1,
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
                            return_type: Some(model::Type::new_api("ReturnType")),
                            attributes: Default::default(),
                        },
                        &Transforms::default(),
                    ),
                    &mut Indented::new(o, INDENT),
                )
            },
            "pub fn rpc_name() -> ReturnType {}\n",
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
                            ty: model::Type::new_api("Type"),
                            attributes: Default::default(),
                        },
                        &vec![],
                        &vec![],
                        &vec![],
                    ),
                    o,
                )
            },
            "asdf: Type,",
        )
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
            "a::b::c",
            model::Type::Api(model::EntityId::from("a.b.c"))
        );

        fn run_test(ty: model::Type, expected: &str) -> Result<()> {
            assert_output(|o| write_type(Type::new(&ty, &vec![]), o), expected)
        }
    }

    #[test]
    fn entity_id() -> Result<()> {
        let entity_id = model::EntityId::from("a.b.c");
        assert_output(
            |o| write_entity_id(view::EntityId::new(&entity_id, &vec![]), o),
            "a::b::c",
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
