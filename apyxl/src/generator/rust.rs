use anyhow::Result;

use crate::generator::Generator;
use crate::model::{Api, Dto, DtoRef, Field, Rpc};
use crate::output::{Indented, Output};

#[derive(Default)]
pub struct Rust {}

const INDENT: &str = "    ";

impl Generator for Rust {
    fn generate(&mut self, api: &Api, output: &mut dyn Output) -> Result<()> {
        let mut o = Indented::new(output, INDENT);

        for rpc in api.rpcs() {
            self.write_rpc(rpc, &mut o)?;
            o.newline()?;
        }

        for dto in api.dtos() {
            self.write_dto(dto, &mut o)?;
            o.newline()?;
        }
        Ok(())
    }
}

impl Rust {
    fn write_dto(&mut self, dto: &Dto, o: &mut Indented) -> Result<()> {
        self.write_dto_start(dto, o)?;

        for field in &dto.fields {
            self.write_field(field, o)?;
            o.newline()?;
        }

        self.write_block_end(o)
    }

    fn write_rpc(&mut self, rpc: &Rpc, o: &mut Indented) -> Result<()> {
        o.write_str("pub fn ")?;
        o.write_str(rpc.name)?;

        o.write('(')?;
        o.indent(1);
        for field in &rpc.params {
            o.newline()?;
            self.write_field(field, o)?;
        }
        o.indent(-1);

        if !rpc.params.is_empty() {
            o.newline()?;
        }

        o.write(')')?;

        if let Some(return_type) = &rpc.return_type {
            o.write_str(" -> ")?;
            self.write_dto_ref(return_type, o)?;
        }

        o.write(';')?;
        o.newline()
    }

    fn write_dto_start(&mut self, dto: &Dto, o: &mut Indented) -> Result<()> {
        o.write_str("struct ")?;
        o.write_str(dto.name)?;
        o.write_str(" {")?;
        o.indent(1);
        o.newline()
    }

    fn write_block_end(&mut self, o: &mut Indented) -> Result<()> {
        o.indent(-1);
        o.write_str("}")?;
        o.newline()?;
        Ok(())
    }

    fn write_field(&mut self, field: &Field, o: &mut dyn Output) -> Result<()> {
        self.write_param(field, o)?;
        o.write(',')?;
        Ok(())
    }

    fn write_param(&mut self, field: &Field, o: &mut dyn Output) -> Result<()> {
        o.write_str(field.name)?;
        o.write_str(": ")?;
        self.write_dto_ref(&field.ty, o)?;
        Ok(())
    }

    fn write_dto_ref(&mut self, dto_ref: &DtoRef, o: &mut dyn Output) -> Result<()> {
        o.write_str(dto_ref.name)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::generator::rust::INDENT;
    use crate::generator::Rust;
    use crate::model::{Api, Dto, DtoRef, Field, Rpc, Segment};
    use crate::output::{Indented, Output};
    use crate::{output, Generator};
    use anyhow::Result;

    #[test]
    fn full_generation() -> Result<()> {
        let api = Api {
            segments: vec![
                Segment::Dto(Dto {
                    name: "DtoName",
                    fields: vec![Field {
                        name: "i",
                        ty: DtoRef { name: "i32" },
                    }],
                }),
                Segment::Rpc(Rpc {
                    name: "rpc_name",
                    params: vec![Field {
                        name: "dto",
                        ty: DtoRef { name: "DtoName" },
                    }],
                    return_type: Some(DtoRef { name: "DtoName" }),
                }),
            ],
        };
        let expected = r#"pub fn rpc_name(
    dto: DtoName,
) -> DtoName;

struct DtoName {
    i: i32,
}

"#;
        assert_output(|gen, o| gen.generate(&api, o), expected)
    }

    #[test]
    fn dto() -> Result<()> {
        assert_output(
            |gen, o| {
                gen.write_dto(
                    &Dto {
                        name: "DtoName",
                        fields: vec![
                            Field {
                                name: "field0",
                                ty: DtoRef { name: "Type0" },
                            },
                            Field {
                                name: "field1",
                                ty: DtoRef { name: "Type1" },
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
            |gen, o| {
                gen.write_rpc(
                    &Rpc {
                        name: "rpc_name",
                        params: vec![
                            Field {
                                name: "param0",
                                ty: DtoRef { name: "Type0" },
                            },
                            Field {
                                name: "param1",
                                ty: DtoRef { name: "Type1" },
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
            |gen, o| {
                gen.write_rpc(
                    &Rpc {
                        name: "rpc_name",
                        params: vec![],
                        return_type: Some(DtoRef { name: "ReturnType" }),
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
            |gen, o| {
                gen.write_field(
                    &Field {
                        name: "asdf",
                        ty: DtoRef { name: "Type" },
                    },
                    o,
                )
            },
            "asdf: Type,",
        )
    }

    #[test]
    fn dto_ref() -> Result<()> {
        assert_output(
            |gen, o| gen.write_dto_ref(&DtoRef { name: "asdf" }, o),
            "asdf",
        )
    }

    fn assert_output<F: Fn(&mut Rust, &mut dyn Output) -> Result<()>>(
        write: F,
        expected: &str,
    ) -> Result<()> {
        let mut output = output::Buffer::default();
        write(&mut Rust::default(), &mut output)?;
        assert_eq!(&output.to_string(), expected);
        Ok(())
    }
}
