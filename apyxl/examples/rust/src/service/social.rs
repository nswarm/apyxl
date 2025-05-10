use crate::service::user;

// feature: type alias
pub type FriendId = user::Id;

#[derive(Default)]
pub struct Friend {
    id: FriendId,
    // feature: vec
    mutuals: Vec<Friend>,
}
