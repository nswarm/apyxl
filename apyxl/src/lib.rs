use anyhow::Result;

pub use crate::executor::Executor;
pub use crate::generator::Generator;
pub use crate::input::Input;
pub use crate::output::Output;
pub use crate::parser::Parser;

mod executor;
mod generator;
mod input;
mod model;
mod output;
mod parser;

pub fn execute() -> Result<()> {
    let input = input::Buffer::new("abc,def,ghi");
    // Executor::default()
    //     .input(&input)
    //     .parser(&parser::Delimited::new(","))
    //     .generator(
    //         &generator::Dbg::default(),
    //         vec![&mut output::StdOut::new("Debug Api:")],
    //     )
    //     .execute()
    Ok(())
}
