use std::borrow::Cow;

use itertools::Itertools;

use crate::model::api::entity::{Entity, EntityType, ToEntity};
use crate::model::attributes::AttributesHolder;
use crate::model::entity::{EntityMut, FindEntity};
use crate::model::{Attributes, Dto, EntityId, Enum, Field, Rpc, TypeAlias};

/// A named, nestable wrapper for a set of API entities.
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct Namespace<'a> {
    pub name: Cow<'a, str>,
    pub children: Vec<NamespaceChild<'a>>,
    pub attributes: Attributes<'a>,

    /// 'virtual' is a temporary namespace indicating it belongs to a [Dto] and should be moved
    /// to the [Dto] at build time. Useful for handling [Rpc]s or other [Dto]s nested inside
    /// or that belong to a [Dto].
    pub is_virtual: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum NamespaceChild<'a> {
    Field(Field<'a>),
    Dto(Dto<'a>),
    Rpc(Rpc<'a>),
    Enum(Enum<'a>),
    TypeAlias(TypeAlias<'a>),
    Namespace(Namespace<'a>),
}

impl ToEntity for Namespace<'_> {
    fn to_entity(&self) -> Entity {
        Entity::Namespace(self)
    }
}

impl AttributesHolder for Namespace<'_> {
    fn attributes(&self) -> &Attributes {
        &self.attributes
    }
}

impl<'api> FindEntity<'api> for Namespace<'api> {
    fn find_entity<'a>(&'a self, mut id: EntityId) -> Option<Entity<'a, 'api>> {
        if let Some((ty, name)) = id.pop_front() {
            match ty {
                EntityType::Namespace => self.namespace(&name).and_then(|x| x.find_entity(id)),
                EntityType::Dto => self.dto(&name).and_then(|x| x.find_entity(id)),
                EntityType::Rpc => self.rpc(&name).and_then(|x| x.find_entity(id)),
                EntityType::Enum => self.en(&name).and_then(|x| x.find_entity(id)),
                EntityType::TypeAlias => self.ty_alias(&name).and_then(|x| x.find_entity(id)),
                EntityType::Field => self.field(&name).and_then(|x| x.find_entity(id)),

                EntityType::None | EntityType::Type => None,
            }
        } else {
            Some(Entity::Namespace(self))
        }
    }

    fn find_entity_mut<'a>(&'a mut self, mut id: EntityId) -> Option<EntityMut<'a, 'api>> {
        if let Some((ty, name)) = id.pop_front() {
            match ty {
                EntityType::Namespace => self
                    .namespace_mut(&name)
                    .and_then(|x| x.find_entity_mut(id)),
                EntityType::Dto => self.dto_mut(&name).and_then(|x| x.find_entity_mut(id)),
                EntityType::Rpc => self.rpc_mut(&name).and_then(|x| x.find_entity_mut(id)),
                EntityType::Enum => self.en_mut(&name).and_then(|x| x.find_entity_mut(id)),
                EntityType::TypeAlias => {
                    self.ty_alias_mut(&name).and_then(|x| x.find_entity_mut(id))
                }
                EntityType::Field => self.field_mut(&name).and_then(|x| x.find_entity_mut(id)),

                EntityType::None | EntityType::Type => None,
            }
        } else {
            Some(EntityMut::Namespace(self))
        }
    }
}

impl<'a> Namespace<'a> {
    /// Perform a simple merge of [Namespace] `other` into this [Namespace] by adding all of
    /// `other`'s children to this [Namespace]'s children. `other`'s name is ignored. This may
    /// result in duplicate children.
    pub fn merge(&mut self, mut other: Namespace<'a>) {
        self.children.append(&mut other.children);
        self.attributes.merge(other.attributes);
    }

    /// Add the [Dto] `dto` as a child of this [Namespace].
    /// No validation is performed to ensure the [Dto] does not already exist, which may result
    /// in duplicates.
    pub fn add_dto(&mut self, dto: Dto<'a>) {
        self.children.push(NamespaceChild::Dto(dto));
    }

    /// Add the [Rpc] `rpc` as a child of this [Namespace].
    /// No validation is performed to ensure the [Rpc] does not already exist, which may result
    /// in duplicates.
    pub fn add_rpc(&mut self, rpc: Rpc<'a>) {
        self.children.push(NamespaceChild::Rpc(rpc));
    }

    /// Add the [Enum] `enum` as a child of this [Namespace].
    /// No validation is performed to ensure the [Enum] does not already exist, which may result
    /// in duplicates.
    pub fn add_enum(&mut self, en: Enum<'a>) {
        self.children.push(NamespaceChild::Enum(en));
    }

    /// Add the [TypeAlias] `ty_alias` as a child of this [Namespace].
    /// No validation is performed to ensure the [TypeAlias] does not already exist, which may result
    /// in duplicates.
    pub fn add_ty_alias(&mut self, alias: TypeAlias<'a>) {
        self.children.push(NamespaceChild::TypeAlias(alias));
    }

    /// Add the [Field] `field` as a child of this [Namespace].
    /// No validation is performed to ensure the [Field] does not already exist, which may result
    /// in duplicates.
    pub fn add_field(&mut self, field: Field<'a>) {
        self.children.push(NamespaceChild::Field(field));
    }

    /// Add the [Namespace] `namespace` as a child of this [Namespace].
    /// No validation is performed to ensure the [Namespace] does not already exist, which may result
    /// in duplicates.
    pub fn add_namespace(&mut self, namespace: Namespace<'a>) {
        self.children.push(NamespaceChild::Namespace(namespace));
    }

    /// Get a [NamespaceChild] within this [Namespace] by name.
    pub fn child(&self, name: &str) -> Option<&NamespaceChild<'a>> {
        self.children.iter().find(|s| s.name() == name)
    }

    /// Get a mutable [NamespaceChild] within this [Namespace] by name.
    pub fn child_mut(&mut self, name: &str) -> Option<&mut NamespaceChild<'a>> {
        self.children.iter_mut().find(|s| s.name() == name)
    }

    /// Get a [Dto] within this [Namespace] by name.
    pub fn dto(&self, name: &str) -> Option<&Dto<'a>> {
        self.children.iter().find_map(|s| match s {
            NamespaceChild::Dto(value) if value.name == name => Some(value),
            _ => None,
        })
    }

    /// Get a mutable [Dto] within this [Namespace] by name.
    pub fn dto_mut(&mut self, name: &str) -> Option<&mut Dto<'a>> {
        self.children.iter_mut().find_map(|s| match s {
            // todo... trait<T> fn that returns Option<T: ChildType>... trait ChildType
            // impl FindChild<Dto> for Namespace { match Dto(x) => Some(x), _ => None }
            NamespaceChild::Dto(value) if value.name == name => Some(value),
            _ => None,
        })
    }

    /// Get an [Rpc] within this [Namespace] by name.
    pub fn rpc(&self, name: &str) -> Option<&Rpc<'a>> {
        self.children.iter().find_map(|s| match s {
            NamespaceChild::Rpc(value) if value.name == name => Some(value),
            _ => None,
        })
    }

    /// Get a mutable [Rpc] within this [Namespace] by name.
    pub fn rpc_mut(&mut self, name: &str) -> Option<&mut Rpc<'a>> {
        self.children.iter_mut().find_map(|s| match s {
            NamespaceChild::Rpc(value) if value.name == name => Some(value),
            _ => None,
        })
    }

    /// Get an [Enum] within this [Namespace] by name.
    pub fn en(&self, name: &str) -> Option<&Enum<'a>> {
        self.children.iter().find_map(|s| match s {
            NamespaceChild::Enum(en) if en.name == name => Some(en),
            _ => None,
        })
    }

    /// Get a mutable [Enum] within this [Namespace] by name.
    pub fn en_mut(&mut self, name: &str) -> Option<&mut Enum<'a>> {
        self.children.iter_mut().find_map(|s| match s {
            NamespaceChild::Enum(en) if en.name == name => Some(en),
            _ => None,
        })
    }

    /// Get a [TypeAlias] within this [Namespace] by name.
    pub fn ty_alias(&self, name: &str) -> Option<&TypeAlias<'a>> {
        self.children.iter().find_map(|s| match s {
            NamespaceChild::TypeAlias(alias) if alias.name == name => Some(alias),
            _ => None,
        })
    }

    /// Get a mutable [TypeAlias] within this [Namespace] by name.
    pub fn ty_alias_mut(&mut self, name: &str) -> Option<&mut TypeAlias<'a>> {
        self.children.iter_mut().find_map(|s| match s {
            NamespaceChild::TypeAlias(alias) if alias.name == name => Some(alias),
            _ => None,
        })
    }

    /// Get a [Field] within this [Namespace] by name.
    pub fn field(&self, name: &str) -> Option<&Field<'a>> {
        self.children.iter().find_map(|s| match s {
            NamespaceChild::Field(value) if value.name == name => Some(value),
            _ => None,
        })
    }

    /// Get a mutable [Field] within this [Namespace] by name.
    pub fn field_mut(&mut self, name: &str) -> Option<&mut Field<'a>> {
        self.children.iter_mut().find_map(|s| match s {
            NamespaceChild::Field(value) if value.name == name => Some(value),
            _ => None,
        })
    }

    /// Get a [Namespace] within this [Namespace] by name.
    pub fn namespace(&self, name: &str) -> Option<&Namespace<'a>> {
        self.children.iter().find_map(|s| match s {
            NamespaceChild::Namespace(value) if value.name == name => Some(value),
            NamespaceChild::Dto(dto) if dto.name == name && dto.namespace.is_some() => {
                Some(dto.namespace.as_ref().unwrap())
            }
            _ => None,
        })
    }

    /// Get a mutable [Namespace] within this [Namespace] by name.
    pub fn namespace_mut(&mut self, name: &str) -> Option<&mut Namespace<'a>> {
        self.children.iter_mut().find_map(|s| match s {
            NamespaceChild::Namespace(value) if value.name == name => Some(value),
            NamespaceChild::Dto(dto) if dto.name == name && dto.namespace.is_some() => {
                Some(dto.namespace.as_mut().unwrap())
            }
            _ => None,
        })
    }

    /// Iterate over all [Dto]s within this [Namespace].
    pub fn dtos(&self) -> impl Iterator<Item = &Dto<'a>> {
        self.children.iter().filter_map(|child| {
            if let NamespaceChild::Dto(value) = child {
                Some(value)
            } else {
                None
            }
        })
    }

    /// Iterate over all [Dto]s mutably within this [Namespace].
    pub fn dtos_mut(&mut self) -> impl Iterator<Item = &mut Dto<'a>> {
        self.children.iter_mut().filter_map(|child| {
            if let NamespaceChild::Dto(value) = child {
                Some(value)
            } else {
                None
            }
        })
    }

    /// Iterate over all [Rpc]s within this [Namespace].
    pub fn rpcs(&self) -> impl Iterator<Item = &Rpc<'a>> {
        self.children.iter().filter_map(|child| {
            if let NamespaceChild::Rpc(value) = child {
                Some(value)
            } else {
                None
            }
        })
    }

    /// Iterate over all [Rpc]s mutably within this [Namespace].
    pub fn rpcs_mut(&mut self) -> impl Iterator<Item = &mut Rpc<'a>> {
        self.children.iter_mut().filter_map(|child| {
            if let NamespaceChild::Rpc(value) = child {
                Some(value)
            } else {
                None
            }
        })
    }

    /// Iterate over all [Enum]s within this [Namespace].
    pub fn enums(&self) -> impl Iterator<Item = &Enum<'a>> {
        self.children.iter().filter_map(|child| {
            if let NamespaceChild::Enum(value) = child {
                Some(value)
            } else {
                None
            }
        })
    }

    /// Iterate over all [Enum]s mutably within this [Namespace].
    pub fn enums_mut(&mut self) -> impl Iterator<Item = &mut Enum<'a>> {
        self.children.iter_mut().filter_map(|child| {
            if let NamespaceChild::Enum(value) = child {
                Some(value)
            } else {
                None
            }
        })
    }

    /// Iterate over all [TypeAlias]s within this [Namespace].
    pub fn ty_aliases(&self) -> impl Iterator<Item = &TypeAlias<'a>> {
        self.children.iter().filter_map(|child| {
            if let NamespaceChild::TypeAlias(value) = child {
                Some(value)
            } else {
                None
            }
        })
    }

    /// Iterate over all [TypeAlias]s mutably within this [Namespace].
    pub fn ty_aliases_mut(&mut self) -> impl Iterator<Item = &mut TypeAlias<'a>> {
        self.children.iter_mut().filter_map(|child| {
            if let NamespaceChild::TypeAlias(value) = child {
                Some(value)
            } else {
                None
            }
        })
    }

    /// Iterate over all [Field]s within this [Namespace].
    pub fn fields(&self) -> impl Iterator<Item = &Field<'a>> {
        self.children.iter().filter_map(|child| {
            if let NamespaceChild::Field(value) = child {
                Some(value)
            } else {
                None
            }
        })
    }

    /// Iterate over all [Field]s mutably within this [Namespace].
    pub fn fields_mut(&mut self) -> impl Iterator<Item = &mut Field<'a>> {
        self.children.iter_mut().filter_map(|child| {
            if let NamespaceChild::Field(value) = child {
                Some(value)
            } else {
                None
            }
        })
    }

    /// Iterate over all [Namespace]s within this [Namespace].
    pub fn namespaces(&self) -> impl Iterator<Item = &Namespace<'a>> {
        self.children.iter().filter_map(|child| {
            if let NamespaceChild::Namespace(value) = child {
                Some(value)
            } else {
                None
            }
        })
    }

    /// Iterate mutably over all [Namespace]s within this [Namespace].
    pub fn namespaces_mut(&mut self) -> impl Iterator<Item = &mut Namespace<'a>> {
        self.children.iter_mut().filter_map(|child| {
            if let NamespaceChild::Namespace(value) = child {
                Some(value)
            } else {
                None
            }
        })
    }

    /// Removes all [Namespaces] that match `include` and return them in a [Vec].
    pub fn take_namespaces_filtered(
        &mut self,
        take: impl Fn(&Namespace<'a>) -> bool,
    ) -> Vec<Namespace<'a>> {
        // todo use drain_filter when stabilized. https://doc.rust-lang.org/std/vec/struct.DrainFilter.html
        let (take, keep) = self.children.drain(..).partition(|child| match child {
            NamespaceChild::Namespace(namespace) => take(namespace),
            _ => false,
        });

        self.children = keep;

        take.into_iter()
            .map(|child| {
                if let NamespaceChild::Namespace(ns) = child {
                    ns
                } else {
                    unreachable!("already checked that it matches")
                }
            })
            .collect_vec()
    }

    /// Removes all [Namespaces] from this [Namespace] and returns them in a [Vec].
    pub fn take_namespaces(&mut self) -> Vec<Namespace<'a>> {
        self.take_namespaces_filtered(|_| true)
    }

    /// Find a [NamespaceChild] by [EntityId] relative to this [Namespace].
    pub fn find_child(&self, entity_id: &EntityId) -> Option<&NamespaceChild<'a>> {
        let namespace = self.find_namespace(&unqualified_namespace(entity_id));
        let name = unqualified_name(entity_id);
        match (namespace, name) {
            (Some(namespace), Some(name)) => namespace.child(name),
            _ => None,
        }
    }

    /// Find a [Dto] by [EntityId] relative to this [Namespace].
    pub fn find_dto(&self, entity_id: &EntityId) -> Option<&Dto<'a>> {
        let namespace = self.find_namespace(&unqualified_namespace(entity_id));
        let name = unqualified_name(entity_id);
        match (namespace, name) {
            (Some(namespace), Some(name)) => namespace.dto(name),
            _ => None,
        }
    }

    /// Find a mutable [Dto] by [EntityId] relative to this [Namespace].
    pub fn find_dto_mut(&mut self, entity_id: &EntityId) -> Option<&mut Dto<'a>> {
        let namespace = self.find_namespace_mut(&unqualified_namespace(entity_id));
        let name = unqualified_name(entity_id);
        match (namespace, name) {
            (Some(namespace), Some(name)) => namespace.dto_mut(name),
            _ => None,
        }
    }

    /// Find a [Rpc] by [EntityId] relative to this [Namespace].
    pub fn find_rpc(&self, entity_id: &EntityId) -> Option<&Rpc<'a>> {
        let namespace = self.find_namespace(&unqualified_namespace(entity_id));
        let name = unqualified_name(entity_id);
        match (namespace, name) {
            (Some(namespace), Some(name)) => namespace.rpc(name),
            _ => None,
        }
    }

    /// Find a mutable [Rpc] by [EntityId] relative to this [Namespace].
    pub fn find_rpc_mut(&mut self, entity_id: &EntityId) -> Option<&mut Rpc<'a>> {
        let namespace = self.find_namespace_mut(&unqualified_namespace(entity_id));
        let name = unqualified_name(entity_id);
        match (namespace, name) {
            (Some(namespace), Some(name)) => namespace.rpc_mut(name),
            _ => None,
        }
    }

    /// Find an [Enum] by [EntityId] relative to this [Namespace].
    pub fn find_enum(&self, entity_id: &EntityId) -> Option<&Enum<'a>> {
        let namespace = self.find_namespace(&unqualified_namespace(entity_id));
        let name = unqualified_name(entity_id);
        match (namespace, name) {
            (Some(namespace), Some(name)) => namespace.en(name),
            _ => None,
        }
    }

    /// Find a mutable [Enum] by [EntityId] relative to this [Namespace].
    pub fn find_enum_mut(&mut self, entity_id: &EntityId) -> Option<&mut Enum<'a>> {
        let namespace = self.find_namespace_mut(&unqualified_namespace(entity_id));
        let name = unqualified_name(entity_id);
        match (namespace, name) {
            (Some(namespace), Some(name)) => namespace.en_mut(name),
            _ => None,
        }
    }

    /// Find a [TypeAlias] by [EntityId] relative to this [Namespace].
    pub fn find_ty_alias(&self, entity_id: &EntityId) -> Option<&TypeAlias<'a>> {
        let namespace = self.find_namespace(&unqualified_namespace(entity_id));
        let name = unqualified_name(entity_id);
        match (namespace, name) {
            (Some(namespace), Some(name)) => namespace.ty_alias(name),
            _ => None,
        }
    }

    /// Find a mutable [TypeAlias] by [EntityId] relative to this [Namespace].
    pub fn find_ty_alias_mut(&mut self, entity_id: &EntityId) -> Option<&mut TypeAlias<'a>> {
        let namespace = self.find_namespace_mut(&unqualified_namespace(entity_id));
        let name = unqualified_name(entity_id);
        match (namespace, name) {
            (Some(namespace), Some(name)) => namespace.ty_alias_mut(name),
            _ => None,
        }
    }

    /// Find a [Field] by [EntityId] relative to this [Namespace].
    pub fn find_field(&self, entity_id: &EntityId) -> Option<&Field<'a>> {
        let namespace = self.find_namespace(&unqualified_namespace(entity_id));
        let name = unqualified_name(entity_id);
        match (namespace, name) {
            (Some(namespace), Some(name)) => namespace.field(name),
            _ => None,
        }
    }

    /// Find a mutable [Field] by [EntityId] relative to this [Namespace].
    pub fn find_field_mut(&mut self, entity_id: &EntityId) -> Option<&mut Field<'a>> {
        let namespace = self.find_namespace_mut(&unqualified_namespace(entity_id));
        let name = unqualified_name(entity_id);
        match (namespace, name) {
            (Some(namespace), Some(name)) => namespace.field_mut(name),
            _ => None,
        }
    }

    /// Find a [Namespace] by [EntityId] relative to this [Namespace].
    /// If the type ref is empty, this [Namespace] will be returned.
    pub fn find_namespace(&self, entity_id: &EntityId) -> Option<&Namespace<'a>> {
        let mut namespace_it = self;
        for name in entity_id.component_names() {
            if let Some(namespace) = namespace_it.namespace(name) {
                namespace_it = namespace;
            } else {
                return None;
            }
        }
        Some(namespace_it)
    }

    /// Find a [Namespace] by [EntityId] relative to this [Namespace].
    pub fn find_namespace_mut(&mut self, entity_id: &EntityId) -> Option<&mut Namespace<'a>> {
        let mut namespace_it = self;
        for name in entity_id.component_names() {
            if let Some(namespace) = namespace_it.namespace_mut(name) {
                namespace_it = namespace;
            } else {
                return None;
            }
        }
        Some(namespace_it)
    }

    pub fn apply_attr_to_children_recursively<F: FnMut(&mut Attributes) + Clone>(
        &mut self,
        mut f: F,
    ) {
        for namespace in self.namespaces_mut() {
            namespace.apply_attr_to_children_recursively(f.clone());
        }
        for child in &mut self.children {
            f(child.attributes_mut())
        }
    }

    pub fn extract_non_static<'b>(&'b mut self) -> (Vec<Field<'a>>, Vec<Rpc<'a>>) {
        let mut fields = vec![];
        let mut rpcs = vec![];
        let children = self.children.drain(..).collect_vec();
        for child in children {
            match child {
                NamespaceChild::Field(field) if !field.is_static => fields.push(field),
                NamespaceChild::Rpc(rpc) if !rpc.is_static => rpcs.push(rpc),
                child => self.children.push(child),
            }
        }
        (fields, rpcs)
    }
}

impl<'a> NamespaceChild<'a> {
    pub fn name(&self) -> &str {
        match self {
            NamespaceChild::Dto(dto) => &dto.name,
            NamespaceChild::Rpc(rpc) => &rpc.name,
            NamespaceChild::Enum(en) => &en.name,
            NamespaceChild::Namespace(namespace) => &namespace.name,
            NamespaceChild::TypeAlias(alias) => &alias.name,
            NamespaceChild::Field(field) => &field.name,
        }
    }

    pub fn attributes(&self) -> &Attributes<'a> {
        match self {
            NamespaceChild::Dto(dto) => &dto.attributes,
            NamespaceChild::Rpc(rpc) => &rpc.attributes,
            NamespaceChild::Enum(en) => &en.attributes,
            NamespaceChild::Namespace(namespace) => &namespace.attributes,
            NamespaceChild::TypeAlias(alias) => &alias.attributes,
            NamespaceChild::Field(field) => &field.attributes,
        }
    }

    pub fn attributes_mut(&mut self) -> &mut Attributes<'a> {
        match self {
            NamespaceChild::Dto(dto) => &mut dto.attributes,
            NamespaceChild::Rpc(rpc) => &mut rpc.attributes,
            NamespaceChild::Enum(en) => &mut en.attributes,
            NamespaceChild::Namespace(namespace) => &mut namespace.attributes,
            NamespaceChild::TypeAlias(alias) => &mut alias.attributes,
            NamespaceChild::Field(field) => &mut field.attributes,
        }
    }

    pub fn entity_type(&self) -> EntityType {
        self.to_entity().ty()
    }
}

impl ToEntity for NamespaceChild<'_> {
    fn to_entity(&self) -> Entity {
        match self {
            NamespaceChild::Dto(dto) => dto.to_entity(),
            NamespaceChild::Rpc(rpc) => rpc.to_entity(),
            NamespaceChild::Enum(en) => en.to_entity(),
            NamespaceChild::Namespace(namespace) => namespace.to_entity(),
            NamespaceChild::TypeAlias(alias) => alias.to_entity(),
            NamespaceChild::Field(field) => field.to_entity(),
        }
    }
}

fn unqualified_name(id: &EntityId) -> Option<&str> {
    if id.len() > 0 {
        id.component_names().last()
    } else {
        None
    }
}

fn unqualified_namespace(id: &EntityId) -> EntityId {
    if id.len() > 1 {
        EntityId::new_unqualified_vec(id.component_names().take(id.len() - 1))
    } else {
        EntityId::default()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::model::attributes::AttributesHolder;
    use crate::model::{chunk, Api, EntityId, Namespace};
    use crate::test_util::executor::TestExecutor;
    use crate::test_util::{
        test_dto, test_enum, test_field, test_namespace, test_rpc, test_ty_alias,
    };

    #[test]
    fn merge() {
        let mut exe0 = TestExecutor::new(
            r#"
            fn rpc0() {}
            struct dto0 {}
            mod nested0 {}
            enum en0 {}
            type alias0 = u32;
            const field0: u32 = 5;
        "#,
        );
        let mut ns0 = exe0.api();

        let mut exe1 = TestExecutor::new(
            r#"
            fn rpc1() {}
            struct dto1 {}
            mod nested1 {}
            enum en1 {}
            type alias1 = u32;
            const field1: u32 = 5;
        "#,
        );
        let ns1 = exe1.api();

        ns0.merge(ns1);
        assert_eq!(ns0.dtos().count(), 2);
        assert_eq!(ns0.rpcs().count(), 2);
        assert_eq!(ns0.namespaces().count(), 2);
        assert_eq!(ns0.enums().count(), 2);
        assert_eq!(ns0.ty_aliases().count(), 2);
        assert_eq!(ns0.fields().count(), 2);
        assert!(ns0.dto("dto0").is_some());
        assert!(ns0.rpc("rpc0").is_some());
        assert!(ns0.namespace("nested0").is_some());
        assert!(ns0.en("en0").is_some());
        assert!(ns0.ty_alias("alias0").is_some());
        assert!(ns0.field("field0").is_some());
        assert!(ns0.dto("dto1").is_some());
        assert!(ns0.rpc("rpc1").is_some());
        assert!(ns0.namespace("nested1").is_some());
        assert!(ns0.en("en1").is_some());
        assert!(ns0.ty_alias("alias1").is_some());
        assert!(ns0.field("field1").is_some());
    }

    mod take_namespaces {
        use crate::test_util::executor::TestExecutor;

        #[test]
        fn removes_all_namespaces() {
            let mut exe = TestExecutor::new(
                r#"
            mod ns0 {}
            struct dto {}
            mod ns1 {}
            fn rpc() {}
        "#,
            );
            let mut ns = exe.api();
            let taken = ns.take_namespaces();
            assert!(ns.dto("ns0").is_none());
            assert!(ns.dto("ns1").is_none());
            assert!(ns.dto("dto").is_some());
            assert!(ns.rpc("rpc").is_some());
            assert_eq!(taken.len(), 2);
            assert_eq!(taken[0].name, "ns0");
            assert_eq!(taken[1].name, "ns1");
        }

        #[test]
        fn filtered() {
            let mut exe = TestExecutor::new(
                r#"
            mod ns {}
            mod remove_me {}
            mod remove_me_jk {}
        "#,
            );
            let mut api = exe.api();
            let taken = api.take_namespaces_filtered(|inner_ns| inner_ns.name == "remove_me");
            assert!(api.namespace("ns").is_some());
            assert!(api.namespace("remove_me_jk").is_some());
            assert!(api.namespace("remove_me").is_none());
            assert_eq!(taken.len(), 1);
            assert_eq!(taken[0].name, "remove_me");
        }
    }

    mod add_get {
        use crate::model::api::namespace::tests::{complex_api, complex_namespace};
        use crate::test_util::{test_dto, test_enum, test_field, test_rpc, test_ty_alias, NAMES};

        macro_rules! test {
            ($name:ident, $get:ident, $get_mut:ident, $create:ident) => {
                #[test]
                fn $name() {
                    let mut api = complex_api();
                    assert_eq!(api.$get(NAMES[1]), Some($create(1)).as_ref());
                    assert_eq!(api.$get(NAMES[2]), Some($create(2)).as_ref());
                    assert_eq!(api.$get_mut(NAMES[1]), Some($create(1)).as_mut());
                    assert_eq!(api.$get_mut(NAMES[2]), Some($create(2)).as_mut());
                }
            };
        }

        test!(dto, dto, dto_mut, test_dto);
        test!(rpc, rpc, rpc_mut, test_rpc);
        test!(namespace, namespace, namespace_mut, complex_namespace);
        test!(en, en, en_mut, test_enum);
        test!(ty_alias, ty_alias, ty_alias_mut, test_ty_alias);
        test!(field, field, field_mut, test_field);
    }

    mod iter {
        use crate::model::api::namespace::tests::{complex_api, complex_namespace};
        use crate::test_util::{test_dto, test_enum, test_field, test_rpc, test_ty_alias};

        macro_rules! test {
            ($name:ident, $create:ident) => {
                #[test]
                fn $name() {
                    let api = complex_api();
                    assert_eq!(
                        api.$name().collect::<Vec<_>>(),
                        vec![&$create(1), &$create(2)]
                    );
                }
            };
        }

        test!(dtos, test_dto);
        test!(rpcs, test_rpc);
        test!(namespaces, complex_namespace);
        test!(enums, test_enum);
        test!(ty_aliases, test_ty_alias);
        test!(fields, test_field);
    }

    mod find {
        use std::borrow::Cow;

        use crate::model::api::namespace::tests::{complex_api, complex_namespace};
        use crate::model::{Api, EntityId, Namespace};
        use crate::test_util::{
            test_dto, test_enum, test_field, test_namespace, test_rpc, test_ty_alias, NAMES,
        };

        macro_rules! test {
            ($name:ident, $find:ident, $find_mut:ident, $create:ident) => {
                #[test]
                fn $name() {
                    let mut api = complex_api();
                    let entity_id1 = EntityId::new_unqualified(&$create(1).name);
                    let entity_id2 = EntityId::new_unqualified(&$create(2).name);
                    assert_eq!(api.$find(&entity_id1), Some(&$create(1)));
                    assert_eq!(api.$find_mut(&entity_id2), Some(&mut $create(2)));
                }
            };
        }

        test!(dto, find_dto, find_dto_mut, test_dto);
        test!(rpc, find_rpc, find_rpc_mut, test_rpc);
        test!(
            namespace,
            find_namespace,
            find_namespace_mut,
            complex_namespace
        );
        test!(en, find_enum, find_enum_mut, test_enum);
        test!(ty_alias, find_ty_alias, find_ty_alias_mut, test_ty_alias);
        test!(field, find_field, find_field_mut, test_field);

        #[test]
        fn dto_namespace() {
            let mut api = Api::default();
            let mut dto = test_dto(1);
            let dto_name = dto.name.to_string();
            dto.namespace = Some(Namespace::default());
            let expected_dto = dto.clone();
            api.add_dto(dto);

            let entity_id = EntityId::new_unqualified(&dto_name);
            assert_eq!(
                api.find_namespace(&entity_id),
                expected_dto.namespace.as_ref()
            );
        }

        #[test]
        fn child() {
            let api = complex_api();
            let entity_id = EntityId::new_unqualified_vec(
                [complex_namespace(1).name, Cow::Borrowed(NAMES[3])].iter(),
            );
            assert_eq!(api.find_dto(&entity_id), Some(&test_dto(3)));
            assert_eq!(api.find_rpc(&entity_id), Some(&test_rpc(3)));
            assert_eq!(api.find_namespace(&entity_id), Some(&test_namespace(3)));
        }

        #[test]
        fn multi_depth_child() {
            let api = complex_api();
            let entity_id = EntityId::new_unqualified_vec(
                [
                    complex_namespace(1).name,
                    test_namespace(4).name,
                    Cow::Borrowed(NAMES[5]),
                ]
                .iter(),
            );
            assert_eq!(api.find_dto(&entity_id), Some(&test_dto(5)));
        }
    }

    mod parent {
        use crate::model::EntityId;

        #[test]
        fn no_parent() {
            let ty = EntityId::default();
            assert_eq!(ty.parent(), None);
        }

        #[test]
        fn parent_is_root() {
            let ty = EntityId::new_unqualified("dto");
            assert_eq!(ty.parent(), Some(EntityId::default()));
        }

        #[test]
        fn typical() {
            let ty = EntityId::new_unqualified("ns0.ns1.dto");
            let parent = ty.parent();
            assert_eq!(parent, Some(EntityId::new_unqualified("ns0.ns1")));
            assert_eq!(
                parent.unwrap().parent(),
                Some(EntityId::new_unqualified("ns0"))
            );
        }
    }

    #[test]
    fn apply_attr_to_children() {
        let mut exe = TestExecutor::new(
            r#"
                    mod ns0 {
                        mod ns1 {
                            struct dto {}
                            fn rpc() {}
                            enum en {}
                            type alias = u32;
                            const field: u32 = 0;
                        }
                        struct dto {}
                        fn rpc() {}
                        enum en {}
                        type alias = u32;
                        const field: u32 = 0;
                    }
                "#,
        );
        let mut api = exe.api();
        let expected_chunk = PathBuf::from("a/b/c");
        api.find_namespace_mut(&EntityId::new_unqualified("ns0"))
            .unwrap()
            .apply_attr_to_children_recursively(|attr| {
                attr.chunk
                    .get_or_insert(chunk::Attribute::default())
                    .relative_file_paths
                    .push(expected_chunk.clone())
            });
        let entity_id = EntityId::new_unqualified("ns0.ns1");
        let expected = vec![expected_chunk.clone()];
        assert_eq!(
            file_paths(api.find_namespace(&entity_id).unwrap()),
            expected
        );
        let entity_id = EntityId::new_unqualified("ns0.dto");
        assert_eq!(file_paths(api.find_dto(&entity_id).unwrap()), expected);
        let entity_id = EntityId::new_unqualified("ns0.ns1.dto");
        assert_eq!(file_paths(api.find_dto(&entity_id).unwrap()), expected);
        let entity_id = EntityId::new_unqualified("ns0.rpc");
        assert_eq!(file_paths(api.find_rpc(&entity_id).unwrap()), expected);
        let entity_id = EntityId::new_unqualified("ns0.ns1.rpc");
        assert_eq!(file_paths(api.find_rpc(&entity_id).unwrap()), expected);
        let entity_id = EntityId::new_unqualified("ns0.en");
        assert_eq!(file_paths(api.find_enum(&entity_id).unwrap()), expected);
        let entity_id = EntityId::new_unqualified("ns0.ns1.en");
        assert_eq!(file_paths(api.find_enum(&entity_id).unwrap()), expected);
        let entity_id = EntityId::new_unqualified("ns0.alias");
        assert_eq!(file_paths(api.find_ty_alias(&entity_id).unwrap()), expected);
        let entity_id = EntityId::new_unqualified("ns0.ns1.alias");
        assert_eq!(file_paths(api.find_ty_alias(&entity_id).unwrap()), expected);
        let entity_id = EntityId::new_unqualified("ns0.field");
        assert_eq!(file_paths(api.find_field(&entity_id).unwrap()), expected);
        let entity_id = EntityId::new_unqualified("ns0.ns1.field");
        assert_eq!(file_paths(api.find_field(&entity_id).unwrap()), expected);
    }

    fn file_paths(holder: &impl AttributesHolder) -> &[PathBuf] {
        &holder
            .attributes()
            .chunk
            .as_ref()
            .unwrap()
            .relative_file_paths
    }

    fn complex_api() -> Api<'static> {
        let mut api = Api::default();
        api.add_dto(test_dto(1));
        api.add_dto(test_dto(2));
        api.add_rpc(test_rpc(1));
        api.add_rpc(test_rpc(2));
        api.add_enum(test_enum(1));
        api.add_enum(test_enum(2));
        api.add_ty_alias(test_ty_alias(1));
        api.add_ty_alias(test_ty_alias(2));
        api.add_field(test_field(1));
        api.add_field(test_field(2));
        api.add_namespace(complex_namespace(1));
        api.add_namespace(complex_namespace(2));
        api
    }

    fn complex_namespace(i: usize) -> Namespace<'static> {
        let mut namespace = test_namespace(i);
        namespace.add_dto(test_dto(i + 2));
        namespace.add_dto(test_dto(i + 3));
        namespace.add_rpc(test_rpc(i + 2));
        namespace.add_rpc(test_rpc(i + 3));
        namespace.add_enum(test_enum(i + 2));
        namespace.add_enum(test_enum(i + 3));
        namespace.add_ty_alias(test_ty_alias(i + 2));
        namespace.add_ty_alias(test_ty_alias(i + 3));
        namespace.add_field(test_field(i + 2));
        namespace.add_field(test_field(i + 3));
        namespace.add_namespace(test_namespace(i + 2));
        let mut deep_namespace = test_namespace(i + 3);
        deep_namespace.add_dto(test_dto(5));
        namespace.add_namespace(deep_namespace);
        namespace
    }
}
