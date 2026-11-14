use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateSpaceRequest {
    /// Name shown to members of the new shared Space.
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct TransferOwnershipRequest {
    pub new_owner_id: Uuid,
}
