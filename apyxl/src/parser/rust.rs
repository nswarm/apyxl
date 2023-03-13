use anyhow::Result;
use chumsky::prelude::*;

use crate::model::Model;
use crate::Input;
use crate::Parser as ApyxlParser;

struct Rust {}

impl ApyxlParser for Rust {
    fn parse(&self, input: &dyn Input) -> Result<Model> {
        // parser().parse(input.data().chars())
        Ok(Model::default())
    }
}

fn field<'a>() -> impl Parser<'a, &'a str, (&'a str, &'a str)> {
    let ty = text::ident().then_ignore(just(',').padded());
    let field = text::ident()
        .then_ignore(just(':').padded())
        .then(ty)
        .padded();

    field
    // recursive(|x| field)
}

// struct Type {
//     field: FieldType,
//     ident colon ident comma
//     field2: Field2Type,
// }

#[cfg(test)]
mod test {
    use crate::parser::rust::field;
    use chumsky::Parser;

    #[test]
    fn test_field() {
        let result = field().parse("name: Type,");
        let output = result.into_output();
        assert_eq!(output, Some(("name", "Type")))
    }
}
