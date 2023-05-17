#[derive(Debug, Default)]
pub struct Config {
    /// Prints the API after merging namespaces, but before validation. Useful for debugging
    /// validation.
    pub debug_pre_validate_print: PreValidatePrint,
}

#[derive(Debug, Default)]
pub enum PreValidatePrint {
    #[default]
    None,

    /// Prints the API using [generator::Rust]. This is more readable than
    /// [PreValidatePrint::Debug], but may be missing information like user-provided attributes.
    Rust,

    /// Print the API using [std::fmt::Debug]. This is very verbose, but complete.
    Debug,
}
