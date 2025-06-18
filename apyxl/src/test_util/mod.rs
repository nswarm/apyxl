use crate::model;
use crate::model::{Semantics, Type, TypeRef};
use std::borrow::Cow;

pub mod executor;

pub const NAMES: &[&str] = &["name0", "name1", "name2", "name3", "name4", "name5"];

pub fn test_namespace(i: usize) -> model::Namespace<'static> {
    model::Namespace {
        name: Cow::Borrowed(NAMES[i]),
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
        name: Cow::Borrowed(NAMES[i]),
        ..Default::default()
    }
}

pub fn test_enum(i: usize) -> model::Enum<'static> {
    model::Enum {
        name: NAMES[i],
        ..Default::default()
    }
}

pub fn test_ty_alias(i: usize) -> model::TypeAlias<'static> {
    model::TypeAlias {
        name: NAMES[i],
        target_ty: TypeRef::new(Type::U32, Semantics::Value),
        attributes: Default::default(),
    }
}

pub fn test_field(i: usize) -> model::Field<'static> {
    model::Field {
        name: NAMES[i],
        ty: TypeRef::new(Type::U32, Semantics::Value),
        attributes: Default::default(),
        is_static: false,
    }
}
