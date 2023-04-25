use itertools::Itertools;
use std::borrow::Cow;
use std::fmt::{Display, Formatter};

/// A reference to another entity within the [Api].
#[derive(Default, Debug, Clone, Eq, PartialEq, Hash)]
pub struct EntityId<'a> {
    /// The path through other entities in the [Api] to get to the referred to entity. This will
    /// typically be a path through the hierarchy of [NamespaceChild], but can also refer to
    /// sub-child items like [Dto] fields or [Rpc] parameters.
    ///
    /// Examples:
    ///     `namespace1.namespace2.DtoName`
    ///     `namespace1.namespace2.DtoName.field0`
    ///     `namespace1.RpcName.param0`
    pub path: Vec<Cow<'a, str>>,
}

/// Equivalent to EntityId except with no internal references. This type is explicitly purely owned
/// for cases when the implicit reference of `Cow` is too restrictive.
#[derive(Default, Debug, Clone, Eq, PartialEq, Hash)]
pub struct OwnedEntityId {
    /// See [EntityId::path].
    pub path: Vec<String>,
}

impl<'a> EntityId<'a> {
    pub fn borrowed<S: AsRef<&'a str>>(path: &[S]) -> Self {
        Self {
            path: path
                .iter()
                .map(AsRef::as_ref)
                .map(|s| *s)
                .map(Cow::Borrowed)
                .collect_vec(),
        }
    }

    pub fn owned<S: ToString>(path: &[S]) -> Self {
        Self {
            path: path
                .iter()
                .map(ToString::to_string)
                .map(Cow::Owned)
                .collect_vec(),
        }
    }

    pub fn to_owned(&self) -> OwnedEntityId {
        OwnedEntityId {
            path: self.path.iter().map(|s| s.to_string()).collect_vec(),
        }
    }

    pub fn parent(&self) -> Option<Self> {
        let path = &self.path;
        if path.is_empty() {
            return None;
        }
        let len = path.len() - 1;
        Some(Self {
            path: path[..len].to_vec(),
        })
    }

    pub fn child(&self, name: &'a str) -> Self {
        let mut child = self.clone();
        child.path.push(Cow::Borrowed(name));
        child
    }

    pub fn has_namespace(&self) -> bool {
        self.path.len() > 1
    }

    /// Returns the part of the path _before_ the name.
    pub fn namespace(&self) -> EntityId<'a> {
        EntityId {
            path: self
                .path
                .clone()
                .into_iter()
                .take(self.path.len() - 1)
                .collect_vec(),
        }
    }

    /// The name is always the last part of the type path.
    pub fn name(&self) -> Option<Cow<'a, str>> {
        self.path.last().cloned()
    }
}

impl<'a, T> From<T> for EntityId<'a>
where
    T: AsRef<[&'a str]>,
{
    fn from(value: T) -> Self {
        Self {
            path: value
                .as_ref()
                .iter()
                .map(|s| *s)
                .map(Cow::Borrowed)
                .collect_vec(),
        }
    }
}

impl<'a, T> From<T> for OwnedEntityId
where
    T: AsRef<[&'a str]>,
{
    fn from(value: T) -> Self {
        Self {
            path: value.as_ref().iter().map(|s| s.to_string()).collect_vec(),
        }
    }
}

impl Display for EntityId<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path.iter().join("."))
    }
}

impl Display for OwnedEntityId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path.iter().join("."))
    }
}
