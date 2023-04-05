// feature: file dependencies
use crate::service;

// todo enum state

// feature: dto
struct PlatformInfo {
    // todo enum state

    // feature: primitives
    is_healthy: bool,
    num_users: u64,
}

// feature: pure rpc
// feature: rpc return type
fn get_platform_info() -> PlatformInfo {
    PlatformInfo {
        is_healthy: true,
        num_users: 1_000_000,
    }
}

// feature: namespace references
// feature: rpc params
pub fn get_user(id: service::user::Id, is_online: bool) -> service::user::User {
    service::user::User::default()
}
