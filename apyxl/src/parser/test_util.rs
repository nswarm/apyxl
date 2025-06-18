use crate::parser::error::Error;
use anyhow::anyhow;
use chumsky::error::Rich;
use chumsky::prelude::custom;
use chumsky::Parser;
use std::sync::atomic::{AtomicU32, Ordering};

pub type TestError = Vec<Rich<'static, char>>;
pub fn wrap_test_err(err: TestError) -> anyhow::Error {
    anyhow!("errors encountered while parsing: {:?}", err)
}

pub fn debug_parser<'a>(msg: impl ToString) -> impl Parser<'a, &'a str, (), Error<'a>> {
    // Only using an atomic for simple mutable static counter...
    static I: AtomicU32 = AtomicU32::new(0);
    custom(move |_| {
        println!(
            "debug_parser[{}]: {}",
            I.load(Ordering::Relaxed),
            msg.to_string()
        );
        I.fetch_add(1, Ordering::Relaxed);
        Ok(())
    })
}
