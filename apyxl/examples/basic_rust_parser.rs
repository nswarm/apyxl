use anyhow::Result;
use apyxl::input;
use apyxl::{generator, output, parser, Executor};

fn main() -> Result<()> {
    env_logger::init();
    let input = input::Buffer::new(
        r#"

        struct String {}

        struct GetDataRequest {
            id: String,
        }

        struct GetDataResponse {
            some_data: some_module::Data,
        }

        mod some_module {
            struct Data {
                value: String,
            }
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
    Executor::new(input, parser::Rust::default())
        .generator(generator::Dbg::default())
        .output(output::StdOut::default())
        .execute()
}
