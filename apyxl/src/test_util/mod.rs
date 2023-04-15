use crate::model;

pub mod executor;

pub const NAMES: &[&str] = &["name0", "name1", "name2", "name3", "name4", "name5"];

pub fn test_namespace(i: usize) -> model::Namespace<'static> {
    model::Namespace {
        name: NAMES[i],
        ..Default::default()
    }
}

pub fn test_dto(i: usize) -> model::Dto<'static> {
    model::Dto {
        name: NAMES[i],
        ..Default::default()
    }
}

pub fn test_rpc(i: usize) -> model::Rpc<'static> {
    model::Rpc {
        name: NAMES[i],
        ..Default::default()
    }
}
