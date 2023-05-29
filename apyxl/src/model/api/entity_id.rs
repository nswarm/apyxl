use std::cmp::Ordering;
use std::collections::VecDeque;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};

use anyhow::{anyhow, Result};
use itertools::{zip_eq, Itertools};

use crate::model::api::entity;
use crate::model::api::entity::EntityType;

/// An [EntityId] is a unique sequence of components that each define the type and name of an
/// entity within the API, and together define a path from through the hierarchy to a specific
/// entity. [EntityId]s are relative to whatever context they are used within, i.e. there is no
/// such thing as an "absolute path" style [EntityId].
///
/// *** Unqualified vs Qualified [EntityId]s ***
///
/// During parsing, [EntityId]s are "unqualified", meaning they do not have type data associated
/// with each component. It's purely a list of names, because parsers can't necessarily know what
/// type of entity the [EntityId] is referencing until the API is complete. Some methods may
/// assert if used on unqualified [EntityId]s. During [crate::model::builder::Builder::build],
/// all [EntityId]s will be qualified.
///
/// *** String Representation ***
///
/// [EntityId] has a string representation which is used to describe it, like so:
///     `namespace1.namespace2.dto:DtoName.field:field_name.ty`
///
/// Each entity within the string is separated by a `.`. Each entity is in the form `subtype:name`,
/// where `subtype` is used to find the relevant [EntityType], and `name` is the parsed name.
/// See below for more on `subtypes`.
///
/// In this example:
///     `ns:namespace1.ns:namespace2.dto:DtoName.field:field_name.ty`
///     `namespace1`        a [crate::model::Namespace]
///     `namespace2`        another [crate::model::Namespace] (inside of `namespace1`)
///     `dto:DtoName`       a [crate::model::Dto] called `DtoName`
///     `field:field_name`  a [crate::model::Field] within the [crate::model::Dto] called `field_name`
///     `ty`                the [crate::model::Type] of the [crate::model::Field] ("nameless" - see below)
///
/// Possible `subtypes` are defined by the parent entity. Some subtypes have aliases for convenience
/// e.g. `n` is equivalent to `namespace`.
///
/// all components prefixing an [EntityId] string that _do not have_ a subtype are parsed as
/// [crate::model::Namespace]s, e.g. `n:aaa.n:bbb.n:ccc.dto:Name` is equivalent to `aaa.bbb.ccc.dto:Name` for
/// readability and convenience.
///
/// Some types are "nameless" in that they can only possibly refer to a single thing in the entity
/// and don't have a user-defined name. e.g. [crate::model::Rpc] `return_ty`.
///
/// Subtypes:
///     <top level>:               `n`, `namespace`, <empty>: [crate::model::Namespace],
///     [crate::model::Namespace]: `d`, `dto`:                [crate::model::Dto],
///                                `r`, `rpc`:                [crate::model::Rpc],
///                                `e`, `enum`, `en`:         [crate::model::Enum],
///     [crate::model::Dto]:       `f`, `field`:              [crate::model::Field],
///     [crate::model::Rpc]:       `p`, `param`:              [crate::model::Field],
///                                `return_ty`:               [crate::model::Type] (nameless),
///     [crate::model::Field]:     `ty`:                      [crate::model::Type] (nameless),
///     [crate::model::Enum]:      <none>
///     [crate::model::Type]:      <none>
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct EntityId {
    components: VecDeque<Component>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Component {
    pub ty: EntityType,
    pub name: String,
}

impl EntityId {
    /// When parsing, you don't necessarily know what type of entity the [EntityId] is referencing.
    /// using `new_unqualified` takes only names, and during [crate::model::builder::Builder::build]
    /// any unqualified [EntityIds] will be qualified when the complete api is at its disposal.
    pub fn new_unqualified(component_names: &str) -> Self {
        Self::new_unqualified_vec(component_names.split('.'))
    }

    pub fn new_unqualified_vec<S: ToString>(component_names: impl Iterator<Item = S>) -> Self {
        Self {
            components: component_names
                .map(|s| s.to_string())
                .map(|name| Component {
                    ty: EntityType::None,
                    name,
                })
                .collect::<VecDeque<_>>(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }

    pub fn len(&self) -> usize {
        self.components.len()
    }

    pub fn is_qualified(&self) -> bool {
        !self.components.iter().any(|c| c.ty == EntityType::None)
    }

    pub fn component_names(&self) -> impl Iterator<Item = &str> {
        self.components
            .iter()
            .map(|component| component.name.as_str())
    }

    /// Returns a qualified copy of this [EntityId] assuming that every component is a namespace.
    pub fn to_qualified_namespaces(&self) -> Self {
        if self.is_qualified() {
            return self.clone();
        }
        let mut qualified = Self::default();
        for name in self.component_names() {
            qualified.components.push_back(Component {
                ty: EntityType::Namespace,
                name: name.to_string(),
            })
        }
        qualified
    }

    /// Returns an unqualified copy of this [EntityId].
    pub fn to_unqualified(&self) -> Self {
        if !self.is_qualified() {
            return self.clone();
        }
        let mut qualified = Self::default();
        for name in self.component_names() {
            qualified.components.push_back(Component {
                ty: EntityType::None,
                name: name.to_string(),
            })
        }
        qualified
    }

    /// Return an [EntityId] of a step up from this [EntityId], if any.
    ///
    /// Unqualified [EntityId]: Callable.
    /// ```
    /// use apyxl::model::EntityId;
    /// let dto = EntityId::try_from("a.dto:Name").unwrap();
    /// let parent = dto.parent().unwrap();
    /// let grandparent = parent.parent().unwrap();
    /// assert_eq!(parent, EntityId::try_from("a").unwrap());
    /// assert_eq!(grandparent, EntityId::default());
    /// assert_eq!(grandparent.parent(), None);
    /// ```
    pub fn parent(&self) -> Option<Self> {
        let components = &self.components;
        if components.len() == 0 {
            return None;
        }
        let mut components = components.clone();
        let _ = components.pop_back();
        Some(Self { components })
    }

    /// Extend the [EntityId] with the given child. Will error if the type is not valid on the
    /// current [EntityId] e.g. trying to attach a [Field] to a [Namespace].
    ///
    /// Unqualified [EntityId]: Callable. `ty` is ignored.
    /// ```
    /// use apyxl::model::{EntityId, EntityType};
    /// let id = EntityId::try_from("a.b").unwrap();
    /// let child = id.child(EntityType::Namespace, "c").unwrap();
    /// assert_eq!(child, EntityId::try_from("a.b.c").unwrap());
    /// ```
    pub fn child<S: ToString>(&self, ty: EntityType, name: S) -> Result<Self> {
        if let Some(last) = self.components.iter().last() {
            if !last.ty.is_valid_subtype(&ty) {
                return Err(anyhow!(
                    "EntityId: '{:?}' is not a valid subtype for {:?}",
                    ty,
                    last.ty
                ));
            }
        }
        let mut child = self.clone();
        child.components.push_back(Component {
            ty,
            name: name.to_string(),
        });
        Ok(child)
    }

    /// Simplification of [EntityId::child] for unqualified ids. Panics if the EntityId is qualified.
    pub fn child_unqualified<S: ToString>(&self, name: S) -> Self {
        self.fail_qualified("child_unqualified");
        self.child(EntityType::None, name).unwrap()
    }

    /// Concat two [EntityId]s into one.
    ///
    /// Unqualified [EntityId]: Callable.
    /// ```
    /// use apyxl::model::EntityId;
    /// let lhs = EntityId::try_from("a.b").unwrap();
    /// let rhs = EntityId::try_from("c.dto:Name").unwrap();
    /// assert_eq!(
    ///     lhs.concat(&rhs).unwrap(),
    ///     EntityId::try_from("a.b.c.dto:Name").unwrap(),
    /// );
    /// ```
    pub fn concat(&self, other: &EntityId) -> Result<Self> {
        let mut id = self.clone();
        for component in &other.components {
            let last_ty = id.components.iter().last().map(|c| c.ty);
            let is_valid_subtype = last_ty
                .map(|ty| ty.is_valid_subtype(&component.ty))
                .unwrap_or(true);
            if is_valid_subtype {
                id.components.push_back(component.clone());
            } else {
                return Err(anyhow!(
                    "EntityId: '{:?}' is not a valid subtype for {:?}",
                    component.ty,
                    last_ty
                ));
            }
        }
        Ok(id)
    }

    /// True if there are namespace entities.
    ///
    /// Unqualified [EntityId]: _Not callable_
    pub fn has_namespace(&self) -> bool {
        self.fail_unqualified("has_namespace");
        self.components
            .iter()
            .any(|c| c.ty == EntityType::Namespace)
    }

    /// Returns the components _before_ the first non-namespace entity, if any.
    ///
    /// Unqualified [EntityId]: _Not callable_
    /// ```
    /// use apyxl::model::EntityId;
    /// let id = EntityId::try_from("a.b.c.dto:Name").unwrap();
    /// assert_eq!(id.namespace().unwrap(), EntityId::try_from("a.b.c").unwrap());
    /// ```
    pub fn namespace(&self) -> Option<EntityId> {
        self.fail_unqualified("namespace");
        let components = self
            .components
            .iter()
            .filter(|c| c.ty == EntityType::Namespace)
            .map(Clone::clone)
            .collect::<VecDeque<_>>();
        if components.is_empty() {
            None
        } else {
            Some(EntityId { components })
        }
    }

    /// Returns an [EntityId] with the components _after_ any namespaces, if nay.
    ///
    /// Unqualified [EntityId]: _Not callable_
    /// ```
    /// use apyxl::model::EntityId;
    /// let id = EntityId::try_from("a.b.c.dto:Name").unwrap();
    /// assert_eq!(id.without_namespace().unwrap(), EntityId::try_from("dto:Name").unwrap());
    /// ```
    pub fn without_namespace(&self) -> Option<EntityId> {
        self.fail_unqualified("without_namespace");
        let components = self
            .components
            .iter()
            .filter(|c| c.ty != EntityType::Namespace)
            .map(Clone::clone)
            .collect::<VecDeque<_>>();
        if components.is_empty() {
            None
        } else {
            Some(EntityId { components })
        }
    }

    /// Removes and returns the first component of this [EntityId].
    /// ```
    /// use apyxl::model::{EntityId, EntityType};
    /// let mut id = EntityId::try_from("a.dto:Name").unwrap();
    ///
    /// assert_eq!(id.pop_front(), Some((EntityType::Namespace, "a".to_string())));
    /// assert_eq!(id, EntityId::try_from("dto:Name").unwrap());
    ///
    /// assert_eq!(id.pop_front(), Some((EntityType::Dto, "Name".to_string())));
    /// assert_eq!(id, EntityId::default());
    ///
    /// assert_eq!(id.pop_front(), None);
    /// ```
    pub fn pop_front(&mut self) -> Option<(EntityType, String)> {
        match self.components.pop_front() {
            None => None,
            Some(component) => Some((component.ty, component.name)),
        }
    }

    fn fail_qualified(&self, name: &str) {
        assert!(
            !self.is_qualified(),
            "EntityId: do not call '{}' on a qualified EntityId",
            name
        );
    }

    fn fail_unqualified(&self, name: &str) {
        assert!(
            self.is_qualified(),
            "EntityId: do not call '{}' on an unqualified EntityId",
            name
        );
    }
}

impl Display for EntityId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut path = vec![];
        let mut last_component = Option::<&Component>::None;
        for component in &self.components {
            match component.ty {
                EntityType::None => path.push(component.name.clone()),
                EntityType::Namespace => path.push(component.name.clone()),
                EntityType::Dto => {
                    path.push(format!("{}:{}", entity::subtype::DTO, component.name))
                }
                EntityType::Rpc => {
                    path.push(format!("{}:{}", entity::subtype::RPC, component.name))
                }
                EntityType::Enum => {
                    path.push(format!("{}:{}", entity::subtype::ENUM, component.name))
                }
                EntityType::Field => {
                    path.push(format!("{}:{}", entity::subtype::FIELD, component.name))
                }
                EntityType::Type => match last_component {
                    Some(c) if c.ty == EntityType::Field => {
                        path.push(entity::subtype::TY.to_owned())
                    }
                    Some(c) if c.ty == EntityType::Rpc => {
                        path.push(entity::subtype::RETURN_TY.to_owned())
                    }
                    _ => return Err(std::fmt::Error),
                },
            }
            last_component = Some(&component);
        }
        write!(f, "{}", path.join("."))
    }
}

impl Hash for EntityId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.components.hash(state)
    }
}

impl<S: AsRef<str>> TryFrom<&[S]> for EntityId {
    type Error = anyhow::Error;

    fn try_from(value: &[S]) -> Result<Self, Self::Error> {
        let mut components = VecDeque::new();
        for s in value.iter().map(AsRef::as_ref) {
            let split = s.split(":").collect_vec();
            let parent = components.iter().last();
            if split.len() < 2 {
                let value = split.get(0).unwrap();
                // Namespaces are allowed without subtype.
                if let Ok(c) =
                    parse_component(entity::subtype::NAMESPACE, value.to_string(), parent)
                {
                    components.push_back(c);
                    continue;
                }
                // "nameless" subtypes are allowed depending on context.
                components.push_back(parse_component(value, value.to_string(), parent)?);
            } else if split.len() == 2 {
                let subtype = split.get(0).unwrap();
                let name = split.get(1).unwrap().to_string();
                components.push_back(parse_component(subtype, name, parent)?);
            } else {
                return Err(anyhow!(
                    "EntityId: component '{}' must be in the form `type:name`",
                    s
                ));
            }
        }
        Ok(Self { components })
    }
}

fn parse_component(subtype: &str, name: String, parent: Option<&Component>) -> Result<Component> {
    let entity_type = EntityType::try_from(subtype)?;
    if let Some(parent) = parent {
        if !parent.ty.is_valid_subtype(&entity_type) {
            return Err(anyhow!(
                "EntityId: '{}' is not a valid subtype for {:?}",
                subtype,
                parent.ty
            ));
        }
    }
    Ok(Component {
        ty: entity_type,
        name,
    })
}

impl TryFrom<&str> for EntityId {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(value.split('.').map(str::to_string).collect_vec())
    }
}

impl<S: AsRef<str>> TryFrom<&Vec<S>> for EntityId {
    type Error = anyhow::Error;

    fn try_from(value: &Vec<S>) -> Result<Self, Self::Error> {
        Self::try_from(value.as_slice())
    }
}

impl<S: AsRef<str>> TryFrom<Vec<S>> for EntityId {
    type Error = anyhow::Error;

    fn try_from(value: Vec<S>) -> Result<Self, Self::Error> {
        Self::try_from(value.as_slice())
    }
}

impl Ord for EntityId {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.components.len() < other.components.len() {
            Ordering::Less
        } else if self.components.len() > other.components.len() {
            Ordering::Greater
        } else {
            for (lhs, rhs) in zip_eq(self.components.iter(), other.components.iter()) {
                if lhs == rhs {
                    continue;
                }
                return lhs.cmp(rhs);
            }
            Ordering::Equal
        }
    }
}

impl PartialOrd for EntityId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Component {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}

impl PartialOrd for Component {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    mod from {
        use crate::model::EntityId;

        #[test]
        fn vec() {
            let _ = EntityId::try_from(vec!["ns0", "ns1", "dto:Name", "field:asdf"]).unwrap();
        }

        #[test]
        fn vec_ref() {
            let _ = EntityId::try_from(&vec!["ns0", "ns1", "dto:Name", "field:asdf"]).unwrap();
        }

        #[test]
        fn slice() {
            let _ =
                EntityId::try_from(["ns0", "ns1", "dto:Name", "field:asdf"].as_slice()).unwrap();
        }

        #[test]
        fn str() {
            let _ = EntityId::try_from("ns0.ns1.dto:Name.field:asdf").unwrap();
        }
    }

    mod ord {
        use crate::model::EntityId;

        #[test]
        fn test() {
            let mut v = vec![
                EntityId::try_from("a.b.c.dto:A.field:X").unwrap(),
                EntityId::try_from("a.b.z.dto:C.field:X").unwrap(),
                EntityId::try_from("a.b.z.dto:B.field:X").unwrap(),
                EntityId::try_from("a.b").unwrap(),
                EntityId::try_from("a.b.c.dto:A.field:Y").unwrap(),
                EntityId::try_from("a.b.z.dto:A.field:X").unwrap(),
                EntityId::try_from("a").unwrap(),
                EntityId::try_from("a.b.c.dto:A.field:Z").unwrap(),
            ];
            v.sort();
            assert_eq!(
                v,
                vec![
                    EntityId::try_from("a").unwrap(),
                    EntityId::try_from("a.b").unwrap(),
                    EntityId::try_from("a.b.c.dto:A.field:X").unwrap(),
                    EntityId::try_from("a.b.c.dto:A.field:Y").unwrap(),
                    EntityId::try_from("a.b.c.dto:A.field:Z").unwrap(),
                    EntityId::try_from("a.b.z.dto:A.field:X").unwrap(),
                    EntityId::try_from("a.b.z.dto:B.field:X").unwrap(),
                    EntityId::try_from("a.b.z.dto:C.field:X").unwrap(),
                ]
            );
        }
    }

    mod hierarchy {
        use crate::model::api::entity::EntityType;
        use crate::model::EntityId;

        #[test]
        fn parent() {
            let abc_dto = EntityId::try_from("a.b.dto:Name.field:asdf").unwrap();
            let abc = abc_dto.parent().unwrap();
            let ab = abc.parent().unwrap();
            let a = ab.parent().unwrap();
            let root = a.parent().unwrap();
            let none = root.parent();
            assert_eq!(
                EntityId::try_from("a.b").unwrap().parent().unwrap(),
                EntityId::try_from("a").unwrap()
            );

            assert_eq!(abc, EntityId::try_from("a.b.dto:Name").unwrap());
            assert_eq!(ab, EntityId::try_from("a.b").unwrap());
            assert_eq!(a, EntityId::try_from("a").unwrap());
            assert_eq!(root, EntityId::default());
            assert!(none.is_none());
        }

        #[test]
        fn child_namespace() {
            let id = EntityId::try_from("a.b").unwrap();
            assert_eq!(
                id.child(EntityType::Namespace, "c").unwrap(),
                EntityId::try_from("a.b.c").unwrap(),
            );
        }

        #[test]
        fn child_subtype() {
            let id = EntityId::try_from("a.b").unwrap();
            let dto = id.child(EntityType::Dto, "c").unwrap();
            let field = dto.child(EntityType::Field, "d").unwrap();
            let ty = field.child(EntityType::Type, "ty").unwrap();
            assert_eq!(dto, EntityId::try_from("a.b.dto:c").unwrap());
            assert_eq!(field, EntityId::try_from("a.b.dto:c.field:d").unwrap());
            assert_eq!(ty, EntityId::try_from("a.b.dto:c.field:d.ty").unwrap());
        }

        #[test]
        fn child_invalid_subtype() {
            assert!(EntityId::try_from("ns")
                .unwrap()
                .child(EntityType::Field, "x")
                .is_err());
            assert!(EntityId::try_from("ns")
                .unwrap()
                .child(EntityType::Type, "x")
                .is_err());
            assert!(EntityId::try_from("dto:x")
                .unwrap()
                .child(EntityType::Rpc, "x")
                .is_err());
            assert!(EntityId::try_from("dto:x")
                .unwrap()
                .child(EntityType::Type, "x")
                .is_err());
            assert!(EntityId::try_from("rpc:x")
                .unwrap()
                .child(EntityType::Dto, "x")
                .is_err());
        }

        #[test]
        fn namespace() {
            let id = EntityId::try_from("a.b.c.dto:Name").unwrap();
            assert_eq!(id.namespace(), Some(EntityId::try_from("a.b.c").unwrap()));
        }

        #[test]
        fn namespace_none() {
            let id = EntityId::try_from("dto:Name").unwrap();
            assert!(id.namespace().is_none());
        }

        #[test]
        fn namespace_solo() {
            let id = EntityId::try_from("a.b.c").unwrap();
            assert_eq!(id.namespace(), Some(id));
        }

        #[test]
        fn has_namespace() {
            let id = EntityId::try_from("a.b.c.dto:Name").unwrap();
            assert!(id.has_namespace());
        }

        #[test]
        fn has_no_namespace() {
            let id = EntityId::try_from("dto:Name").unwrap();
            assert!(!id.has_namespace());
        }

        #[test]
        fn concat_invalid() {
            let lhs = EntityId::try_from("dto:Name").unwrap();
            let rhs = EntityId::try_from("a.b.c").unwrap();
            assert!(lhs.concat(&rhs).is_err());
        }
    }

    mod display {
        use anyhow::Result;

        use crate::model::EntityId;

        #[test]
        fn with_ty() -> Result<()> {
            run_test("a.b.c.d:Name.f:name.ty", "a.b.c.dto:Name.field:name.ty")
        }

        #[test]
        fn with_return_ty() -> Result<()> {
            run_test("a.b.c.r:Name.return_ty", "a.b.c.rpc:Name.return_ty")
        }

        fn run_test(from: &str, expected: &str) -> Result<()> {
            let display = format!("{}", EntityId::try_from(from)?);
            assert_eq!(&display, expected);
            Ok(())
        }
    }
}
