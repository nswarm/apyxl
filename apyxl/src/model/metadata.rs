use crate::model::chunk;

#[derive(Debug, Default)]
pub struct Metadata {
    pub chunks: Vec<chunk::Metadata>,
}
