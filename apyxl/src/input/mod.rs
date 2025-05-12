use crate::model::Chunk;
pub use buffer::Buffer;
pub use chunk_buffer::ChunkBuffer;
pub use file_set::FileSet;
pub use glob::Glob;
pub use stdin::StdIn;

mod buffer;
mod chunk_buffer;
mod file_set;
mod glob;
mod stdin;

/// An [Input] wraps some form of data retrieval and translates it to the format
/// required by an apyxl [crate].
///
/// [Input] is built around the idea that data will come from a series of [Chunk]s, typically
/// referring to individual files in a set of input files. [Chunk]s and their associated [Data]
/// must remain in memory for the duration of parsing. This is a choice that requires more memory,
/// but allows the parsing and generation process to be nearly copy-free.
pub trait Input {
    fn chunks(&self) -> Vec<(&Chunk, &Data)>;
}

pub type Data = String;
