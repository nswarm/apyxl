use crate::service::user;

#[derive(Default)]
pub struct Friend {
    id: user::Id,
    // feature: vec
    mutuals: Vec<Friend>,
}
