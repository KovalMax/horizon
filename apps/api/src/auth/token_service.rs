use uuid::Uuid;

use super::models::AccessPrincipal;

pub trait TokenService: Send + Sync {
    fn issue_access_token(&self, user_id: Uuid, session_id: Uuid) -> Result<String, String>;
    fn verify_access_token(&self, token: &str) -> Option<AccessPrincipal>;
    fn create_secret(&self) -> String;
    fn hash_secret(&self, secret: &str) -> Vec<u8>;
}
