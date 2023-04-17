pub use buffer::Buffer;
pub use file_set::FileSet;
pub use glob::Glob;
pub use stdin::StdIn;

use crate::model::chunk;

mod buffer;
mod file_set;
mod glob;
mod stdin;

/// An [Input] wraps some form of data retrieval and translates it to the format
/// required by an apyxl [crate::Parser].
///
/// [Input] is built around the idea that data will come from a series of [Chunk]s, typically
/// referring to individual files in a set of input files. Chunks must remain in memory for the
/// duration of parsing. This is a choice that requires more memory, but allows the parsing and
/// generation process to be nearly copy-free.
pub trait Input {
    /// This will be called when the parser is ready to parse the next [Chunk].
    fn next_chunk(&self) -> Option<&Chunk>;
    // todo (String, &model::Chunk)?
}

/// A section of data to be parser by a [crate::Parser]. The simplest and probably most common
/// example of a [Chunk] is a file.
///
/// Each [Chunk] must parse to a valid [crate::model::Api].
#[derive(Default)]
pub struct Chunk {
    /// The data that should be parsed by the [crate::Parser].
    pub data: String,

    /// Information stored about the chunk.
    pub chunk: chunk::Chunk,
}
