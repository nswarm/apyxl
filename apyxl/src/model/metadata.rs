use crate::model::chunk;
use serde::Serialize;

#[derive(Debug, Default, Serialize)]
pub struct Metadata {
    pub chunks: Vec<chunk::Metadata>,
}
