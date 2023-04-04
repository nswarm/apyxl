mod buffer;
mod file_set;
mod stdin;

pub use buffer::Buffer;
pub use file_set::FileSet;
pub use stdin::StdIn;

/// An [Input] wraps some form of data retrieval and translates it to the format
/// required by an apyxl [crate::parser::Parser].
///
/// [Input] is built around the idea that data will come from a series of `chunks`.
/// The simplest and probably most common example of a `chunk` is when reading in multiple files.
///
/// Each chunk must parse to a valid [crate::model::Api], i.e. this isn't a streaming data
/// handler. Instead it assumes it's fine to hold each `chunk` in memory as we parse it. Once
/// all `chunks` are parse in [crate::model::Api]s, they are merged together to form the final
/// [crate::model::Api].
///
/// Importantly, because the result of the parser (the model) holds slice references to the input
/// chunks, they cannot be dropped once returned by next_chunk. This is an intentional decision.
/// While it forces input to keep everything in memory, it means there are no copies involved in
/// parsing or generating by default.
pub trait Input {
    /// This will be called when the parser is ready to parse the next `chunk`.
    fn next_chunk(&self) -> Option<&str>;
}
