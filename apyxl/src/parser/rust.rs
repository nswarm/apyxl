use anyhow::Result;
use chumsky::prelude::*;

use crate::model::{Dto, Field, Model};
use crate::Input;
use crate::Parser as ApyxlParser;

struct Rust {}

impl ApyxlParser for Rust {
    fn parse(&self, input: &dyn Input) -> Result<Model> {
        // parser().parse(input.data().chars())
        Ok(Model::default())
    }
}

fn field<'a>() -> impl Parser<'a, &'a str, Field> {
    // todo type can't be ident (e.g. generics vec/map)
    // todo package pathing
    // todo reference one or more other types (and be able to cross ref that in model)
    let ty = text::ident();
    let field = text::ident()
        .then_ignore(just(':').padded())
        .then(ty)
        .padded();
    field
        .map(|(name, ty): (&str, &str)| (name.to_owned(), ty.to_owned()))
        .map(|(name, ty)| Field { name, ty })
}

fn dto<'a>() -> impl Parser<'a, &'a str, Dto> {
    let fields = field()
        .separated_by(just(',').padded())
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(just('{').padded(), just('}').padded());
    let name = text::keyword("struct").padded().ignore_then(text::ident());
    let dto = name.then(fields);
    dto.map(|(name, fields)| Dto {
        name: name.to_owned(),
        fields,
    })
}

#[cfg(test)]
mod test {
    use crate::parser::rust::{dto, field};
    use chumsky::error::EmptyErr;
    use chumsky::Parser;

    #[test]
    fn test_field() -> Result<(), Vec<EmptyErr>> {
        let result = field().parse("name: Type");
        let output = result.into_result()?;
        assert_eq!(output.name, "name");
        assert_eq!(output.ty, "Type");
        Ok(())
    }

    #[test]
    fn test_dto() -> Result<(), Vec<EmptyErr>> {
        let dto = dto()
            .parse(
                r#"
        struct StructName {
            field0: i32,
            field1: f32,
        }
        "#,
            )
            .into_result()?;
        assert_eq!(&dto.name, "StructName");
        assert_eq!(dto.fields.len(), 2);
        assert_eq!(&dto.fields[0].name, "field0");
        assert_eq!(&dto.fields[1].name, "field1");
        Ok(())
    }
}
