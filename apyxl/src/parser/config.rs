use crate::model::UserTypeName;
use serde::{Deserialize, Serialize};

/// Configuration passed to the parser. These need to be implemented by each individual
/// [crate::parser::Parser] implementation to be supported. Be sure to document if one of the
/// built-in configs is not supported.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    /// See [UserType].
    #[serde(default)]
    pub user_types: Vec<UserType>,
    /// If true, the parser will include private dtos, rpcs, etc. in the API.
    #[serde(default)]
    pub enable_parse_private: bool,
}

/// When the `parse` string is seen by a [crate::parser::Parser], it is mapped to a
/// [crate::model::Type::User] variant with the value `name`.
#[derive(Debug, Serialize, Deserialize)]
pub struct UserType {
    pub parse: String,
    pub name: UserTypeName,
}
