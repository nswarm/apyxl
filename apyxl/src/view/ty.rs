use std::fmt::Debug;

use crate::model;
use crate::model::Semantics;
use crate::view::{EntityId, EntityIdTransform};

pub type InnerType<'v, 'a> = model::BaseType<EntityId<'v>, &'a str>;

#[derive(Debug, Copy, Clone)]
pub struct Type<'v> {
    target: &'v model::Type,
    xforms: &'v Vec<Box<dyn EntityIdTransform>>,
}

impl<'v> Type<'v> {
    pub fn new(target: &'v model::Type, xforms: &'v Vec<Box<dyn EntityIdTransform>>) -> Self {
        Self { target, xforms }
    }

    pub fn inner(&self) -> InnerType {
        self.model_to_view_ty(self.target)
    }

    fn model_to_view_ty<'a>(&'a self, ty: &'a model::Type) -> InnerType {
        match ty {
            model::Type::Bool => InnerType::Bool,
            model::Type::U8 => InnerType::U8,
            model::Type::U16 => InnerType::U16,
            model::Type::U32 => InnerType::U32,
            model::Type::U64 => InnerType::U64,
            model::Type::U128 => InnerType::U128,
            model::Type::USIZE => InnerType::USIZE,
            model::Type::I8 => InnerType::I8,
            model::Type::I16 => InnerType::I16,
            model::Type::I32 => InnerType::I32,
            model::Type::I64 => InnerType::I64,
            model::Type::I128 => InnerType::I128,
            model::Type::F8 => InnerType::F8,
            model::Type::F16 => InnerType::F16,
            model::Type::F32 => InnerType::F32,
            model::Type::F64 => InnerType::F64,
            model::Type::F128 => InnerType::F128,
            model::Type::String => InnerType::String,
            model::Type::Bytes => InnerType::Bytes,
            model::Type::User(name) => InnerType::User(name),
            model::Type::Api(id, semantics) => {
                InnerType::Api(EntityId::new(id, self.xforms), *semantics)
            }
            model::Type::Array(ty) => InnerType::Array(Box::new(self.model_to_view_ty(ty))),
            model::Type::Map { key, value } => InnerType::Map {
                key: Box::new(self.model_to_view_ty(key)),
                value: Box::new(self.model_to_view_ty(value)),
            },
            model::Type::Optional(ty) => InnerType::Optional(Box::new(self.model_to_view_ty(ty))),
        }
    }
}

impl InnerType<'_, '_> {
    pub fn api(&self) -> Option<(&EntityId, Semantics)> {
        if let InnerType::Api(id, semantics) = self {
            Some((id, *semantics))
        } else {
            None
        }
    }
}
