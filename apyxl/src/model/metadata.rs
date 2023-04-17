use crate::model::chunk;

#[derive(Debug, Default)]
pub struct Metadata<'a> {
    pub chunks: Vec<chunk::Metadata<'a>>,
}
