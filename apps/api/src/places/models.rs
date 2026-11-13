use uuid::Uuid;

#[derive(Clone, Debug)]
pub(super) struct SavedPlace {
    pub id: Uuid,
    pub space_id: Uuid,
    pub name: String,
    pub category: String,
    pub address: String,
    pub note: Option<String>,
    pub space_name: String,
    pub space_kind: String,
}

#[derive(Clone, Debug)]
pub(super) struct SavedPlaceDraft {
    pub name: String,
    pub category: String,
    pub address: String,
    pub note: Option<String>,
}

#[derive(Debug)]
pub enum PlaceError {
    Invalid,
    Forbidden,
    NotFound,
    Unavailable,
}
