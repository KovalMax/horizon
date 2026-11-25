use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct PlacesPageQuery {
    /// Maximum number of Saved places to return (defaults to 50, maximum 100).
    #[param(minimum = 1, maximum = 100)]
    pub limit: Option<i64>,
    /// Number of Saved places to skip.
    #[param(minimum = 0, maximum = 1000000)]
    pub offset: Option<i64>,
    /// Restrict results and category counts to this Space.
    pub space_id: Option<Uuid>,
    /// Filter the page by a built-in Place category.
    pub category: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreatePlaceRequest {
    pub name: String,
    pub category: String,
    pub address: String,
    pub note: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct PlaceImportRequest {
    #[schema(max_length = 4096)]
    pub url: String,
}
