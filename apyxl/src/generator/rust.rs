use anyhow::Result;

use crate::generator::Generator;
use crate::model::{Api, Dto, DtoRef, Field};
use crate::output::{Indented, Output};

#[derive(Default)]
pub struct Rust {}

const INDENT: &str = "    ";

impl Generator for Rust {
    fn generate(&mut self, api: &Api, output: &mut dyn Output) -> Result<()> {
        let mut output = Indented::new(output, INDENT);
        for dto in api.dtos() {
            self.write_dto(dto, &mut output)?;
        }
        Ok(())
    }
}

impl Rust {
    fn write_dto(&mut self, dto: &Dto, output: &mut Indented) -> Result<()> {
        self.write_dto_start(dto, output)?;

        for field in &dto.fields {
            self.write_field(field, output)?;
        }

        self.write_dto_end(output)?;
        Ok(())
    }

    fn write_dto_start(&mut self, dto: &Dto, output: &mut Indented) -> Result<()> {
        output.write_str("struct ")?;
        output.write_str(dto.name)?;
        output.write_str(" {")?;
        output.indent(1);
        output.newline()?;
        Ok(())
    }

    fn write_dto_end(&mut self, output: &mut Indented) -> Result<()> {
        output.indent(-1);
        output.write_str("}")?;
        output.newline()?;
        Ok(())
    }

    fn write_field(&mut self, field: &Field, output: &mut dyn Output) -> Result<()> {
        self.write_param(field, output)?;
        output.write(',')?;
        output.newline()?;
        Ok(())
    }

    fn write_param(&mut self, field: &Field, output: &mut dyn Output) -> Result<()> {
        output.write_str(field.name)?;
        output.write_str(": ")?;
        self.write_dto_ref(&field.ty, output)?;
        Ok(())
    }

    fn write_dto_ref(&mut self, dto_ref: &DtoRef, output: &mut dyn Output) -> Result<()> {
        output.write_str(dto_ref.name)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::generator::rust::INDENT;
    use crate::generator::Rust;
    use crate::model::{Dto, DtoRef, Field};
    use crate::output;
    use crate::output::{Indented, Output};
    use anyhow::Result;

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
            "asdf: Type,\n",
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
