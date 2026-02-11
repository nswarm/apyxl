use crate::parser::model::Chunk;
use ariadne::{Color, Label, Report, ReportKind, Source};
use chumsky::error::Rich;
use chumsky::extra;
use std::fmt;

pub type Error<'a, I: fmt::Display> = extra::Err<Rich<'a, I>>;

pub fn report<I: fmt::Display>(chunk: &Chunk, src: &str, errors: Vec<Rich<'_, I>>) {
    let filename = chunk
        .relative_file_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or("unknown".to_string());
    for error in errors {
        Report::build(
            ReportKind::Error,
            (filename.clone(), error.span().into_range()),
        )
        .with_message(error.to_string())
        .with_label(
            Label::new((filename.clone(), error.span().into_range()))
                .with_message(error.reason().to_string())
                .with_color(Color::Red),
        )
        // need "label" feature in chumsky
        // .with_labels(error.contexts().map(|(label, span)| {
        //     Label::new((filename.clone(), span.into_range()))
        //         .with_message(format!("while parsing this {}", label))
        //         .with_color(Color::Yellow)
        // }))
        .finish()
        .print((filename.clone(), Source::from(src)))
        .unwrap()
    }
}
