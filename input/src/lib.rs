mod stdin;

pub use stdin::StdIn;

/// An [Input] wraps some form of data retrieval and translates it to the format
/// required by an apyxl [Parser].
pub trait Input {
    fn data(&self) -> &str;
}
