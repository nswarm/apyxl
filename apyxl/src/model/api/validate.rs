use crate::model::{Api, Field, Namespace, TypeRef, UNDEFINED_NAMESPACE};
use anyhow::anyhow;
use itertools::Itertools;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use thiserror::Error;

#[derive(Error, Debug, Eq, PartialEq)]
pub enum ValidationError<'a> {
    #[error(
        "Invalid namespace found at path {0}. Only the root namespace can be named {}.",
        UNDEFINED_NAMESPACE
    )]
    InvalidNamespaceName(TypeRef<'a>),

    #[error("Invalid DTO name within namespace '{0}', index #{1}. DTO names cannot be empty.")]
    InvalidDtoName(TypeRef<'a>, usize),

    #[error("Invalid RPC name within namespace '{0}', index #{1}. RPC names cannot be empty.")]
    InvalidRpcName(TypeRef<'a>, usize),

    #[error("Invalid field name at '{0}', index {1}. Field names cannot be empty.")]
    InvalidFieldName(TypeRef<'a>, usize),

    #[error(
        "Invalid field type '{0}::{1}', index {2}. Type '{3}' must be a valid DTO in the API."
    )]
    InvalidFieldType(TypeRef<'a>, &'a str, usize, TypeRef<'a>),

    #[error("Invalid return type for RPC {0}. Type '{1}' must be a valid DTO in the API.")]
    InvalidRpcReturnType(TypeRef<'a>, TypeRef<'a>),

    #[error("Duplicate DTO definition: '{0}'")]
    DuplicateDto(TypeRef<'a>),

    #[error("Duplicate RPC definition: '{0}'")]
    DuplicateRpc(TypeRef<'a>),
}

pub fn namespace_names<'a, 'b>(
    _: &'b Api<'a>,
    namespace: &'b Namespace<'a>,
    type_ref: TypeRef<'a>,
) -> impl Iterator<Item = ValidationError<'a>> + 'b {
    namespace.namespaces().filter_map(move |child| {
        if child.name == UNDEFINED_NAMESPACE {
            Some(ValidationError::InvalidNamespaceName(
                type_ref.child(child.name),
            ))
        } else {
            None
        }
    })
}

pub fn no_duplicate_dtos<'a, 'b>(
    _: &'b Api<'a>,
    namespace: &'b Namespace<'a>,
    type_ref: TypeRef<'a>,
) -> impl Iterator<Item = ValidationError<'a>> + 'b {
    namespace
        .dtos()
        .duplicates_by(|dto| dto.name)
        .map(move |dto| ValidationError::DuplicateDto(type_ref.child(dto.name)))
}

pub fn no_duplicate_rpcs<'a, 'b>(
    _: &'b Api<'a>,
    namespace: &'b Namespace<'a>,
    type_ref: TypeRef<'a>,
) -> impl Iterator<Item = ValidationError<'a>> + 'b {
    namespace
        .rpcs()
        .duplicates_by(|rpc| rpc.name)
        .map(move |rpc| ValidationError::DuplicateRpc(type_ref.child(rpc.name)))
}

pub fn dto_names<'a, 'b>(
    _: &'b Api<'a>,
    namespace: &'b Namespace<'a>,
    type_ref: TypeRef<'a>,
) -> impl Iterator<Item = ValidationError<'a>> + 'b {
    namespace.dtos().enumerate().filter_map(move |(i, dto)| {
        if dto.name.is_empty() {
            Some(ValidationError::InvalidDtoName(type_ref.clone(), i))
        } else {
            None
        }
    })
}

pub fn dto_field_names<'a, 'b>(
    api: &'b Api<'a>,
    namespace: &'b Namespace<'a>,
    type_ref: TypeRef<'a>,
) -> impl Iterator<Item = ValidationError<'a>> + 'b {
    namespace
        .dtos()
        .flat_map(move |dto| field_names(api, dto.fields.iter(), type_ref.child(dto.name)))
}

pub fn dto_field_types<'a, 'b>(
    api: &'b Api<'a>,
    namespace: &'b Namespace<'a>,
    type_ref: TypeRef<'a>,
) -> impl Iterator<Item = ValidationError<'a>> + 'b {
    namespace
        .dtos()
        .flat_map(move |dto| field_types(api, dto.fields.iter(), type_ref.child(dto.name)))
}

pub fn rpc_names<'a, 'b>(
    _: &'b Api<'a>,
    namespace: &'b Namespace<'a>,
    type_ref: TypeRef<'a>,
) -> impl Iterator<Item = ValidationError<'a>> + 'b {
    namespace.rpcs().enumerate().filter_map(move |(i, rpc)| {
        if rpc.name.is_empty() {
            Some(ValidationError::InvalidRpcName(type_ref.clone(), i))
        } else {
            None
        }
    })
}

pub fn rpc_param_names<'a, 'b>(
    api: &'b Api<'a>,
    namespace: &'b Namespace<'a>,
    type_ref: TypeRef<'a>,
) -> impl Iterator<Item = ValidationError<'a>> + 'b {
    namespace
        .rpcs()
        .flat_map(move |rpc| field_names(api, rpc.params.iter(), type_ref.child(rpc.name)))
}

pub fn rpc_param_types<'a, 'b>(
    api: &'b Api<'a>,
    namespace: &'b Namespace<'a>,
    type_ref: TypeRef<'a>,
) -> impl Iterator<Item = ValidationError<'a>> + 'b {
    namespace
        .rpcs()
        .flat_map(move |rpc| field_types(api, rpc.params.iter(), type_ref.child(rpc.name)))
}

pub fn rpc_return_types<'a, 'b>(
    api: &'b Api<'a>,
    namespace: &'b Namespace<'a>,
    type_ref: TypeRef<'a>,
) -> impl Iterator<Item = ValidationError<'a>> + 'b {
    namespace
        .rpcs()
        .filter_map(|rpc| rpc.return_type.as_ref().map(|ty| (rpc.name, ty)))
        .filter_map(move |(rpc_name, return_type)| {
            if api.find_dto(return_type).is_none() {
                Some(ValidationError::InvalidRpcReturnType(
                    type_ref.child(rpc_name),
                    return_type.clone(),
                ))
            } else {
                None
            }
        })
}

pub fn field_names<'a, 'b>(
    _: &'b Api<'a>,
    fields: impl Iterator<Item = &'b Field<'a>> + 'b,
    type_ref: TypeRef<'a>,
) -> impl Iterator<Item = ValidationError<'a>> + 'b {
    fields.enumerate().filter_map(move |(i, field)| {
        if field.name.is_empty() {
            Some(ValidationError::InvalidFieldName(type_ref.clone(), i))
        } else {
            None
        }
    })
}

pub fn field_types<'a, 'b>(
    api: &'b Api<'a>,
    fields: impl Iterator<Item = &'b Field<'a>> + 'b,
    parent_type_ref: TypeRef<'a>,
) -> impl Iterator<Item = ValidationError<'a>> + 'b {
    fields.enumerate().filter_map(move |(i, field)| {
        if api.find_dto(&field.ty).is_none() {
            Some(ValidationError::InvalidFieldType(
                parent_type_ref.clone(),
                field.name,
                i,
                field.ty.clone(),
            ))
        } else {
            None
        }
    })
}
