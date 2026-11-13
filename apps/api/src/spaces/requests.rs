use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateSpaceRequest {
    /// Name shown to members of the new shared Space.
    pub name: String,
}
