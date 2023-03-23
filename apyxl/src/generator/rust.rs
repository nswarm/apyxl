use anyhow::Result;

use crate::generator::Generator;
use crate::model::{Api, Dto, Field, Namespace, Rpc, TypeRef};
use crate::output::{Indented, Output};

#[derive(Default)]
pub struct Rust {}

const INDENT: &str = "    ";

impl Generator for Rust {
    fn generate(&mut self, api: &Api, output: &mut dyn Output) -> Result<()> {
        let mut o = Indented::new(output, INDENT);
        write_namespace_contents(api, &mut o)
    }
}

fn write_namespace(namespace: &Namespace, o: &mut Indented) -> Result<()> {
    o.write_str("pub mod ")?;
    o.write_str(namespace.name)?;
    o.write(' ')?;
    write_block_start(o)?;
    write_namespace_contents(namespace, o)?;
    write_block_end(o)
}

fn write_namespace_contents(namespace: &Namespace, o: &mut Indented) -> Result<()> {
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

fn write_dto(dto: &Dto, o: &mut Indented) -> Result<()> {
    write_dto_start(dto, o)?;

    for field in &dto.fields {
        write_field(field, o)?;
        o.newline()?;
    }

    write_block_end(o)
}

fn write_rpc(rpc: &Rpc, o: &mut Indented) -> Result<()> {
    o.write_str("pub fn ")?;
    o.write_str(rpc.name)?;

    o.write('(')?;
    o.indent(1);
    for field in &rpc.params {
        o.newline()?;
        write_field(field, o)?;
    }
    o.indent(-1);

    if !rpc.params.is_empty() {
        o.newline()?;
    }

    o.write(')')?;

    if let Some(return_type) = &rpc.return_type {
        o.write_str(" -> ")?;
        write_dto_ref(return_type, o)?;
    }

    o.write(';')?;
    o.newline()
}

fn write_dto_start(dto: &Dto, o: &mut Indented) -> Result<()> {
    o.write_str("struct ")?;
    o.write_str(dto.name)?;
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

fn write_field(field: &Field, o: &mut dyn Output) -> Result<()> {
    write_param(field, o)?;
    o.write(',')
}

fn write_param(field: &Field, o: &mut dyn Output) -> Result<()> {
    o.write_str(field.name)?;
    o.write_str(": ")?;
    write_dto_ref(&field.ty, o)
}

fn write_dto_ref(dto_ref: &TypeRef, o: &mut dyn Output) -> Result<()> {
    o.write_str(dto_ref.name)
}

#[cfg(test)]
mod test {
    use crate::generator::rust::{write_dto, write_dto_ref, write_field, write_rpc, INDENT};
    use crate::generator::Rust;
    use crate::model::{Api, Dto, Field, Namespace, Rpc, Segment, TypeRef, ROOT_NAMESPACE};
    use crate::output::{Indented, Output};
    use crate::{output, Generator};
    use anyhow::Result;

    #[test]
    fn full_generation() -> Result<()> {
        let api = Api {
            name: ROOT_NAMESPACE,
            segments: vec![
                Segment::Dto(Dto {
                    name: "DtoName",
                    fields: vec![Field {
                        name: "i",
                        ty: TypeRef { name: "i32" },
                    }],
                }),
                Segment::Rpc(Rpc {
                    name: "rpc_name",
                    params: vec![Field {
                        name: "dto",
                        ty: TypeRef { name: "DtoName" },
                    }],
                    return_type: Some(TypeRef { name: "DtoName" }),
                }),
                Segment::Namespace(Namespace {
                    name: "ns0",
                    segments: vec![Segment::Dto(Dto {
                        name: "DtoName",
                        fields: vec![Field {
                            name: "i",
                            ty: TypeRef { name: "i32" },
                        }],
                    })],
                }),
            ],
        };
        let expected = r#"pub fn rpc_name(
    dto: DtoName,
) -> DtoName;

struct DtoName {
    i: i32,
}

pub mod ns0 {
    struct DtoName {
        i: i32,
    }

}

"#;
        assert_output(|o| Rust::default().generate(&api, o), expected)
    }

    #[test]
    fn dto() -> Result<()> {
        assert_output(
            |o| {
                write_dto(
                    &Dto {
                        name: "DtoName",
                        fields: vec![
                            Field {
                                name: "field0",
                                ty: TypeRef { name: "Type0" },
                            },
                            Field {
                                name: "field1",
                                ty: TypeRef { name: "Type1" },
                            },
                        ],
                    },
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
                    &Rpc {
                        name: "rpc_name",
                        params: vec![
                            Field {
                                name: "param0",
                                ty: TypeRef { name: "Type0" },
                            },
                            Field {
                                name: "param1",
                                ty: TypeRef { name: "Type1" },
                            },
                        ],
                        return_type: None,
                    },
                    &mut Indented::new(o, INDENT),
                )
            },
            r#"pub fn rpc_name(
    param0: Type0,
    param1: Type1,
);
"#,
        )
    }

    #[test]
    fn rpc_with_return() -> Result<()> {
        assert_output(
            |o| {
                write_rpc(
                    &Rpc {
                        name: "rpc_name",
                        params: vec![],
                        return_type: Some(TypeRef { name: "ReturnType" }),
                    },
                    &mut Indented::new(o, INDENT),
                )
            },
            "pub fn rpc_name() -> ReturnType;\n",
        )
    }

    #[test]
    fn field() -> Result<()> {
        assert_output(
            |o| {
                write_field(
                    &Field {
                        name: "asdf",
                        ty: TypeRef { name: "Type" },
                    },
                    o,
                )
            },
            "asdf: Type,",
        )
    }

    #[test]
    fn dto_ref() -> Result<()> {
        assert_output(|o| write_dto_ref(&TypeRef { name: "asdf" }, o), "asdf")
    }

    fn assert_output<F: Fn(&mut dyn Output) -> Result<()>>(write: F, expected: &str) -> Result<()> {
        let mut output = output::Buffer::default();
        write(&mut output)?;
        assert_eq!(&output.to_string(), expected);
        Ok(())
    }
}
