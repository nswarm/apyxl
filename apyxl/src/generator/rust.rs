use anyhow::Result;

use crate::generator::indent::Indent;
use crate::generator::Generator;
use crate::model::{Api, Dto, DtoRef, Field};
use crate::output::Output;

#[derive(Default)]
pub struct Rust {
    indent: Indent,
}

impl Generator for Rust {
    fn generate(&mut self, api: &Api, output: &mut dyn Output) -> Result<()> {
        for dto in api.dtos() {
            self.write_dto(dto, output)?;
        }
        Ok(())
    }
}

impl Rust {
    fn write_dto(&mut self, dto: &Dto, output: &mut dyn Output) -> Result<()> {
        self.write_dto_start(dto, output)?;

        for field in &dto.fields {
            self.write_field(field, output)?;
        }

        self.write_dto_end(output)?;
        Ok(())
    }

    fn write_dto_start(&mut self, dto: &Dto, output: &mut dyn Output) -> Result<()> {
        output.write_str("struct ")?;
        output.write_str(dto.name)?;
        output.write_str(" {")?;
        self.indent.add(1);
        Ok(())
    }

    fn write_dto_end(&mut self, output: &mut dyn Output) -> Result<()> {
        self.indent.sub(1);
        output.write_str("}")?;
        Ok(())
    }

    fn write_field(&mut self, field: &Field, output: &mut dyn Output) -> Result<()> {
        self.write_param(field, output)?;
        output.write(',')?;
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
    use crate::generator::Rust;
    use crate::model::{DtoRef, Field};
    use crate::output;
    use crate::output::Output;
    use anyhow::Result;

    #[test]
    fn dto_ref() -> Result<()> {
        assert_output(
            |gen, o| gen.write_dto_ref(&DtoRef { name: "asdf" }, o),
            "asdf",
        )
    }

    #[test]
    fn param() -> Result<()> {
        assert_output(
            |gen, o| {
                gen.write_param(
                    &Field {
                        name: "asdf",
                        ty: DtoRef { name: "Type" },
                    },
                    o,
                )
            },
            "asdf: Type",
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
