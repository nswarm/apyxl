mod buffer;
mod file_set;
mod glob;
mod stdin;

pub use buffer::Buffer;
pub use file_set::FileSet;
pub use glob::Glob;
use std::path::PathBuf;
pub use stdin::StdIn;

/// An [Input] wraps some form of data retrieval and translates it to the format
/// required by an apyxl [crate::Parser].
///
/// [Input] is built around the idea that data will come from a series of [Chunk]s, typically
/// referring to individual files in a set of input files. Chunks must remain in memory for the
/// duration of parsing. This is a choice that requires more memory, but allows the parsing process
/// to be close nearly copy-free until the [crate::Model] is finalized.
pub trait Input {
    /// This will be called when the parser is ready to parse the next [Chunk].
    fn next_chunk(&self) -> Option<&Chunk>;
}

/// A section of data to be parser by a [crate::Parser]. The simplest and probably most common
/// example of a [Chunk] is a file.
///
/// Each [Chunk] must parse to a valid [crate::model::Api].
#[derive(Default)]
pub struct Chunk {
    /// The data that should be parsed by the [crate::Parser].
    pub data: String,

    /// Relative path including file name from a common root path shared by the other [Chunk]s from
    /// the [Input]. Typically used by a [crate::Generator] to determine where to put the final file
    /// for this data, and how to refer to it from other files for includes/imports.
    pub relative_file_path: Option<PathBuf>,
}
