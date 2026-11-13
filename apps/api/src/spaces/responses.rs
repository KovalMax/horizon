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
