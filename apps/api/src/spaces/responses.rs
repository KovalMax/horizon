use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct SpaceResponse {
    pub id: Uuid,
    pub name: String,
    /// `personal` or `shared`.
    pub kind: String,
    /// The current member's role: `owner`, `editor`, or `viewer`.
    pub role: String,
    pub member_count: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct TransferOwnershipResponse {
    pub space_id: Uuid,
    pub previous_owner_id: Uuid,
    pub new_owner_id: Uuid,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct DeletedSpaceResponse {
    pub id: Uuid,
    pub name: String,
    /// `personal` or `shared`.
    pub kind: String,
    pub deleted_at: String,
    pub restore_until: String,
}
