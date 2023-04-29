#[derive(Default)]
pub struct User {
    id: Id,
    display: Display,
}

#[derive(Default)]
struct Display {
    display_name: String,
    discriminator: String,
}

// pub type Id = u128; todo aliases?
#[derive(Default)]
pub struct Id {
    // value: u128,
}

// todo methods on dto types
// impl User {
//
// }
