// feature: file dependencies
use crate::service;

// feature: namespace-level fields
const VERSION: &str = env!("CARGO_PKG_VERSION");

// feature: dto
pub struct PlatformInfo {
    // feature: primitives
    is_healthy: bool,
    num_users: u64,
    user: service::user::User,
}

// feature: pure rpc
// feature: rpc return type
fn get_platform_info() -> PlatformInfo {
    PlatformInfo {
        is_healthy: true,
        num_users: 1_000_000,
        user: Default::default(),
    }
}

impl PlatformInfo {
    // feature: &self
    // feature: namespace references
    // feature: rpc params
    // feature: entity id ref semantics
    pub fn get_user(&self, id: service::user::Id, is_online: bool) -> &service::user::User {
        &self.user
    }

    // feature: &mut self
    // feature: entity id mut semantics
    pub fn get_user_mut(
        &mut self,
        id: service::user::Id,
        is_online: bool,
    ) -> &mut service::user::User {
        &mut self.user
    }
}
