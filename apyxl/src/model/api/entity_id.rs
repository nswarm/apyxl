use itertools::Itertools;
use std::fmt::{Display, Formatter};

/// A reference to another entity within the [Api].
#[derive(Default, Debug, Clone, Eq, PartialEq, Hash)]
pub struct EntityId {
    /// The path through other entities in the [Api] to get to the referred to entity. This will
    /// typically be a path through the hierarchy of [NamespaceChild], but can also refer to
    /// sub-child items like [Dto] fields or [Rpc] parameters.
    ///
    /// Examples:
    ///     `namespace1.namespace2.DtoName`
    ///     `namespace1.namespace2.DtoName.field0`
    ///     `namespace1.RpcName.param0`
    pub path: Vec<String>,
}

impl EntityId {
    pub fn new<T, S>(value: T) -> Self
    where
        S: ToString,
        T: AsRef<[S]>,
    {
        Self {
            path: value.as_ref().iter().map(|s| s.to_string()).collect_vec(),
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

    pub fn child<S: ToString>(&self, name: S) -> Self {
        let mut child = self.clone();
        child.path.push(name.to_string());
        child
    }

    pub fn has_namespace(&self) -> bool {
        self.path.len() > 1
    }

    /// Returns the part of the path _before_ the name.
    pub fn namespace(&self) -> EntityId {
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
    pub fn name(&self) -> Option<&str> {
        self.path.last().map(|s| s.as_str())
    }
}

impl Display for EntityId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path.iter().join("."))
    }
}
