use anyhow::anyhow;
use chumsky::error::Rich;

pub type TestError = Vec<Rich<'static, char>>;
pub fn wrap_test_err(err: TestError) -> anyhow::Error {
    anyhow!("errors encountered while parsing: {:?}", err)
}
