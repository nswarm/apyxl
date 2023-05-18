#[derive(Default)]
pub struct User {
    id: Id,
    display: Display,

    // feature: user type in parser config
    special_id: SpecialId,
}

pub enum Presence {
    Offline,
    Online,
    Invalid = 999,
}

#[derive(Default)]
pub struct Display {
    display_name: String,
    discriminator: String,
    presence: Presence,
}

// pub type Id = u128; todo aliases?
#[derive(Default)]
pub struct Id {
    value: u128,
}

// todo methods on dto types
// impl User {
//
// }
