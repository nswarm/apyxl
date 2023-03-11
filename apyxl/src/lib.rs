use anyhow::Result;

use crate::generator::Generator;
use crate::parser::Parser;

mod generator;
mod input;
mod model;
mod output;
mod parser;

// pub struct Config {
//     // specify:
//     // input
//     // parser
//     // generator
//     // output
// }

pub fn execute() -> Result<()> {
    let input = input::Buffer::new("abc,def,ghi");
    let output = output::StdOut::default();

    // todo builder pattern might fit better here?
    // executor
    //      .input(input::StdIn::new()?)
    //      .parser(parser::Delimited, config)
    //      .generator(generator::Dbg, config)
    //      .output(output::StdOut::default())
    //      .execute();

    let model = parser::Delimited::new(",").parse(&input)?;
    generator::Dbg::default().generate(&model, output)?;

    Ok(())
}
