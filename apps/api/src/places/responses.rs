use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::models::SavedPlace;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PlaceResponse {
    pub id: Uuid,
    pub space_id: Uuid,
    pub space_name: String,
    /// `personal` or `shared`.
    pub space_kind: String,
    pub name: String,
    pub category: String,
    pub address: String,
    pub note: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PlacePageResponse {
    pub items: Vec<PlaceResponse>,
    pub next_offset: Option<i64>,
}

impl From<SavedPlace> for PlaceResponse {
    fn from(place: SavedPlace) -> Self {
        Self {
            id: place.id,
            space_id: place.space_id,
            space_name: place.space_name,
            space_kind: place.space_kind,
            name: place.name,
            category: place.category,
            address: place.address,
            note: place.note,
        }
    }
}
