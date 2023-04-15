use crate::view;
pub use api::*;
pub use builder::Builder;
pub use metadata::Metadata;

pub mod api;
mod builder;
pub mod metadata;

/// In-memory representation of a fully parsed and validated API.
#[derive(Debug, Default)]
pub struct Model<'a> {
    pub api: Api<'a>,
    pub metadata: Metadata<'a>,
}

impl Model<'_> {
    pub fn view(&self) -> view::Model {
        view::Model::new(self)
    }
}
