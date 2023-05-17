use crate::model::UserTypeName;

#[derive(Debug, Default)]
pub struct Config {
    /// See [UserType].
    pub user_types: Vec<UserType>,
}

/// When the `parse` string is seen by a [crate::parser::Parser], it is mapped to a
/// [crate::model::Type::User] variant with the value `name`. This needs to be implemented by
/// the [crate::parser::Parser] implementation itself.
#[derive(Debug)]
pub struct UserType {
    pub parse: String,
    pub name: UserTypeName,
}
