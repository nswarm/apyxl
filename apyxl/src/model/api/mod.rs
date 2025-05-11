pub use attributes::Attributes;
pub use attributes::Comment;
pub use dependencies::Dependencies;
pub use dto::Dto;
pub use en::Enum;
pub use en::EnumValue;
pub use en::EnumValueNumber;
pub use entity::Entity;
pub use entity::EntityType;
pub use entity_id::EntityId;
pub use field::Field;
pub use namespace::Namespace;
pub use namespace::NamespaceChild;
pub use rpc::Rpc;
pub use ty::BaseType;
pub use ty::Semantics;
pub use ty::Type;
pub use ty::TypeRef;
pub use ty::UserTypeName;
pub use ty_alias::TypeAlias;
pub use validate::ValidationError;

pub mod attributes;
mod dependencies;
mod dto;
mod en;
pub mod entity;
mod entity_id;
mod field;
mod namespace;
mod rpc;
mod ty;
mod ty_alias;
pub mod validate;

/// The root namespace of the entire API.
pub const UNDEFINED_NAMESPACE: &str = "_";

/// A complete set of entities that make up an API.
pub type Api<'a> = Namespace<'a>;

impl Api<'_> {
    /// Find `find_ty` by walking up the namespace hierarchy, starting at `initial_namespace`.
    /// Returns the fully qualified type [EntityId] if it exists.
    /// Only supports finding [Dto]s and [Enum]s.
    pub fn find_qualified_type_relative(
        &self,
        initial_namespace: &EntityId,
        find_ty: &EntityId,
    ) -> Option<EntityId> {
        let mut iter = initial_namespace.to_qualified_namespaces();
        loop {
            match self.find_namespace(&iter) {
                None => return None,
                Some(namespace) => {
                    let is_dto = namespace.find_dto(find_ty).is_some();
                    let is_enum = namespace.find_enum(find_ty).is_some();
                    let is_alias = namespace.find_ty_alias(find_ty).is_some();
                    let is_field = namespace.find_field(find_ty).is_some();
                    if is_dto || is_enum | is_alias | is_field {
                        let mut id = iter;
                        let len = find_ty.len();
                        for (i, name) in find_ty.component_names().enumerate() {
                            let ty = if i < len - 1 {
                                EntityType::Namespace
                            } else if is_dto {
                                EntityType::Dto
                            } else if is_enum {
                                EntityType::Enum
                            } else if is_alias {
                                EntityType::TypeAlias
                            } else {
                                EntityType::Field
                            };
                            // unwrap ok: the find^ calls verify it already.
                            id = id.child(ty, name).unwrap();
                        }
                        return Some(id);
                    }
                }
            }
            iter = iter.parent()?
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::model::EntityId;
    use crate::test_util::executor::TestExecutor;

    #[test]
    fn dto_from_root() {
        let initial_namespace = EntityId::default();
        let find_id = EntityId::new_unqualified("ns0.dto");
        run_test(
            r#"
            mod ns0 {
                struct dto {}
            }
            "#,
            &initial_namespace,
            &find_id,
            Some(EntityId::try_from("ns0.d:dto").unwrap()),
        );
    }

    #[test]
    fn dto_from_ns() {
        let initial_namespace = EntityId::new_unqualified("ns0");
        let find_id = EntityId::new_unqualified("ns1.dto");
        run_test(
            r#"
            mod ns0 {
                mod ns1 {
                    struct dto {}
                }
            }
            "#,
            &initial_namespace,
            &find_id,
            Some(EntityId::try_from("ns0.ns1.d:dto").unwrap()),
        );
    }

    #[test]
    fn dto_from_sibling() {
        let initial_namespace = EntityId::new_unqualified("ns0.other");
        let find_id = EntityId::new_unqualified("ns1.dto");
        run_test(
            r#"
            mod ns0 {
                mod ns1 {
                    struct dto {}
                }
                mod other {
                }
            }
            "#,
            &initial_namespace,
            &find_id,
            Some(EntityId::try_from("ns0.ns1.d:dto").unwrap()),
        );
    }

    #[test]
    fn dto_overqualified() {
        let initial_namespace = EntityId::new_unqualified("ns0.ns1.ns2.other");
        let find_id = EntityId::new_unqualified("ns1.ns2.dto");
        run_test(
            r#"
            mod ns0 {
                mod ns1 {
                    mod ns2 {
                        struct dto {}
                        mod other {}
                    }
                }
            }
            "#,
            &initial_namespace,
            &find_id,
            Some(EntityId::try_from("ns0.ns1.ns2.d:dto").unwrap()),
        );
    }

    #[test]
    fn enum_from_ns() {
        let initial_namespace = EntityId::new_unqualified("ns0");
        let find_id = EntityId::new_unqualified("ns1.en");
        run_test(
            r#"
            mod ns0 {
                mod ns1 {
                    enum en {}
                }
            }
            "#,
            &initial_namespace,
            &find_id,
            Some(EntityId::try_from("ns0.ns1.e:en").unwrap()),
        );
    }

    #[test]
    fn ty_alias_from_ns() {
        let initial_namespace = EntityId::new_unqualified("ns0");
        let find_id = EntityId::new_unqualified("ns1.alias");
        run_test(
            r#"
            mod ns0 {
                mod ns1 {
                    type alias = u32;
                }
            }
            "#,
            &initial_namespace,
            &find_id,
            Some(EntityId::try_from("ns0.ns1.a:alias").unwrap()),
        );
    }

    #[test]
    fn field_from_ns() {
        let initial_namespace = EntityId::new_unqualified("ns0");
        let find_id = EntityId::new_unqualified("ns1.field");
        run_test(
            r#"
            mod ns0 {
                mod ns1 {
                    const field: u32 = 5;
                }
            }
            "#,
            &initial_namespace,
            &find_id,
            Some(EntityId::try_from("ns0.ns1.f:field").unwrap()),
        );
    }

    #[test]
    fn does_not_exist() {
        let initial_namespace = EntityId::default();
        let find_id = EntityId::new_unqualified("asdf.dto");
        run_test(
            r#"
            mod ns0 {
                struct dto {}
            }
            "#,
            &initial_namespace,
            &find_id,
            None,
        );
    }

    fn run_test(
        data: &str,
        initial_namespace: &EntityId,
        find_ty: &EntityId,
        expected: Option<EntityId>,
    ) {
        let mut exe = TestExecutor::new(data);
        let api = exe.api();
        assert_eq!(
            api.find_qualified_type_relative(initial_namespace, find_ty),
            expected,
        );
    }
}
