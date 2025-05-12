use crate::service::social;
use std::collections::HashMap;

#[derive(Default)]
pub struct User {
    id: Id,
    display: Display,

    // feature: maps
    // feature: nested type dependency - generator will import social.rs
    friends: HashMap<Id, social::Friend>,

    // feature: complex nested types
    equipment_slots: HashMap<String, Option<Vec<inventory::Item>>>,

    // feature: user type in parser config
    special_id: SpecialId,
}

#[derive(Default)]
pub enum Presence {
    #[default]
    Offline,
    Online,
    Invalid = 999,
}

// todo impl Default for Presence

#[derive(Default)]
pub struct Display {
    display_name: String,
    discriminator: String,
    presence: Presence,
    // feature: optionals
    motd: Option<String>,
}

// feature: these are ignored
pub const PUB_CONST_NAME: &str = "name";
const CONST_NAME: &str = "name";

// feature: type alias to primitive
pub type PubTypeAlias = u128;
type TypeAlias = u128;

#[derive(Default)]
pub struct Id {
    value: u128,
}

// feature: nested types. In this rust case, this is rpcs in a relative to a dto.
impl User {
    // `&self` is ignored in this context since the rpc is stored as a namespace under the dto.
    pub fn name(&self) -> &Display {
        &self.display
    }
}

// feature: nested namespace in file
pub mod inventory {
    #[derive(Default)]
    pub struct Item {
        id: String,
        data: String,
    }
}
