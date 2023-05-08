use crate::view;
pub use api::*;
pub use builder::Builder;
pub use chunk::Chunk;
pub use metadata::Metadata;

pub mod api;
pub mod builder;
pub mod chunk;
pub mod metadata;

/// In-memory representation of a fully parsed and validated API.
#[derive(Debug, Default)]
pub struct Model<'a> {
    api: Api<'a>,
    metadata: Metadata,
    dependencies: Dependencies,
}

impl<'a> Model<'a> {
    pub fn new(api: Api<'a>, metadata: Metadata) -> Self {
        let mut model = Self {
            api,
            metadata,
            dependencies: Default::default(),
        };
        model.dependencies.build(&model.api);
        model
    }

    #[cfg(test)]
    pub fn without_deps(api: Api<'a>, metadata: Metadata) -> Self {
        Self {
            api,
            metadata,
            dependencies: Default::default(),
        }
    }

    pub fn api(&self) -> &Api {
        &self.api
    }

    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    pub fn view(&self) -> view::Model {
        view::Model::new(self)
    }
}
