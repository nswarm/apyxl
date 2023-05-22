pub use attributes::Attributes;
pub use dependencies::Dependencies;
pub use dto::Dto;
pub use en::Enum;
pub use en::EnumValue;
pub use en::EnumValueNumber;
pub use entity_id::EntityId;
pub use field::Field;
pub use namespace::Namespace;
pub use namespace::NamespaceChild;
pub use rpc::Rpc;
pub use ty::BaseType;
pub use ty::Type;
pub use ty::UserTypeName;
pub use validate::ValidationError;

mod attributes;
mod dependencies;
mod dto;
mod en;
mod entity;
mod entity_id;
mod field;
mod namespace;
mod rpc;
mod ty;
pub mod validate;

/// The root namespace of the entire API.
pub const UNDEFINED_NAMESPACE: &str = "_";

/// A complete set of entities that make up an API.
pub type Api<'a> = Namespace<'a>;

impl Api<'_> {
    /// Find `find_ty` by walking up the namespace hierarchy, starting at `initial_namespace`.
    /// Returns the fully qualified type id if it exists.
    pub fn find_type_relative(
        &self,
        initial_namespace: EntityId,
        find_ty: &EntityId,
    ) -> Option<EntityId> {
        let mut iter = initial_namespace;
        loop {
            let namespace = self.find_namespace(&iter);
            match namespace {
                None => return None,
                Some(namespace) => {
                    if namespace.find_dto(find_ty).is_some()
                        || namespace.find_enum(find_ty).is_some()
                    {
                        return Some(iter.concat(find_ty));
                    }
                }
            }
            iter = match iter.parent() {
                None => return None,
                Some(id) => id,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn find_type_relative() {
        todo!()
    }
}
