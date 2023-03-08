mod stdin;

pub use stdin::StdIn;

pub trait Input {
    fn data(&self) -> &str;
}
