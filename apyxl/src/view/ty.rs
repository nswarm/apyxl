use std::fmt::Debug;

use crate::model;
use crate::model::Semantics;
use crate::view::{EntityId, EntityIdTransform, Namespace};

pub type Type<'v, 'a> = model::BaseType<TypeRef<'v>, EntityId<'v>, &'a str>;

#[derive(Debug, Copy, Clone)]
pub struct TypeRef<'v> {
    target: &'v model::TypeRef,
    xforms: &'v Vec<Box<dyn EntityIdTransform>>,
}

impl<'v> TypeRef<'v> {
    pub fn new(target: &'v model::TypeRef, xforms: &'v Vec<Box<dyn EntityIdTransform>>) -> Self {
        Self { target, xforms }
    }

    pub fn nested(&self, target: &'v model::TypeRef) -> Self {
        Self {
            target,
            xforms: self.xforms,
        }
    }

    pub fn value(&self) -> Type {
        self.model_to_view_ty(self.target)
    }

    pub fn semantics(&self) -> Semantics {
        self.target.semantics
    }

    pub fn is_primitive(&self, api: &Namespace) -> bool {
        match &self.value() {
            Type::Api(api_ty) => {
                if let Some(ty) = api.find_ty_alias(api_ty.target()) {
                    ty.target_ty().is_primitive(api)
                } else {
                    false
                }
            }

            Type::Bool
            | Type::U8
            | Type::U16
            | Type::U32
            | Type::U64
            | Type::U128
            | Type::USIZE
            | Type::I8
            | Type::I16
            | Type::I32
            | Type::I64
            | Type::I128
            | Type::F8
            | Type::F16
            | Type::F32
            | Type::F64
            | Type::F128
            | Type::StringView => true,

            Type::String
            | Type::Bytes
            | Type::User(_)
            | Type::Array(_)
            | Type::Map { .. }
            | Type::Optional(_) => false,
        }
    }

    fn model_to_view_ty<'a>(&'a self, ty: &'a model::TypeRef) -> Type<'a, 'a> {
        match &ty.value {
            model::Type::Bool => Type::Bool,
            model::Type::U8 => Type::U8,
            model::Type::U16 => Type::U16,
            model::Type::U32 => Type::U32,
            model::Type::U64 => Type::U64,
            model::Type::U128 => Type::U128,
            model::Type::USIZE => Type::USIZE,
            model::Type::I8 => Type::I8,
            model::Type::I16 => Type::I16,
            model::Type::I32 => Type::I32,
            model::Type::I64 => Type::I64,
            model::Type::I128 => Type::I128,
            model::Type::F8 => Type::F8,
            model::Type::F16 => Type::F16,
            model::Type::F32 => Type::F32,
            model::Type::F64 => Type::F64,
            model::Type::F128 => Type::F128,
            model::Type::StringView => Type::StringView,
            model::Type::String => Type::String,
            model::Type::Bytes => Type::Bytes,
            model::Type::User(name) => Type::User(name),
            model::Type::Api(id) => Type::Api(EntityId::new(id, self.xforms)),
            model::Type::Array(array_ty) => Type::Array(Box::new(self.nested(array_ty))),
            model::Type::Map { key, value } => Type::Map {
                key: Box::new(self.nested(key)),
                value: Box::new(self.nested(value)),
            },
            model::Type::Optional(ty) => Type::Optional(Box::new(self.nested(ty))),
        }
    }
}

impl Type<'_, '_> {
    pub fn api(&self) -> Option<&EntityId> {
        if let Type::Api(id) = self {
            Some(id)
        } else {
            None
        }
    }
}
