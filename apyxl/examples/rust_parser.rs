use anyhow::Result;
use apyxl::input;
use apyxl::{generator, output, parser, Executor};

fn main() -> Result<()> {
    let input = input::Buffer::new(
        r#"
        struct GetDataRequest {
            id: String,
        }

        struct GetDataResponse {
            some_data: Data,
        }

        struct Data {
            value: String,
        }

        fn get_data(user_id: String, request: GetDataRequest) -> GetDataResponse {
            none
            of
            {{this}}
            {
                matters
            }
        }
        "#,
    );
    Executor::default()
        .input(&input)
        .parser(&parser::Rust::default())
        .generator(
            &mut generator::Dbg::default(),
            vec![&mut output::StdOut::default()],
        )
        .execute()
}
