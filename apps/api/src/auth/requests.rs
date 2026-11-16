use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MagicLinkRequest {
    #[schema(format = Email, max_length = 320)]
    pub email: String,
    #[serde(default)]
    pub time_zone: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RedeemMagicLinkRequest {
    #[schema(min_length = 32, write_only)]
    pub token: String,
}
