use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreatePlaceRequest {
    pub name: String,
    pub category: String,
    pub address: String,
    pub note: Option<String>,
}
