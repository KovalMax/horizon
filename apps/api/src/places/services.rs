use sqlx::PgPool;
use uuid::Uuid;

use crate::pagination::{Page, PageWindow};

use super::{
    models::{PlaceError, SavedPlaceDraft},
    repositories::PostgresPlacesRepository,
    requests::CreatePlaceRequest,
    responses::{PlacePageResponse, PlaceResponse},
};

const PLACE_CATEGORIES: [&str; 4] = ["Food & drink", "Groceries", "Health", "Services"];

#[derive(Clone)]
pub struct PlacesApplication {
    repository: PostgresPlacesRepository,
}

impl PlacesApplication {
    pub fn new(pool: PgPool) -> Self {
        Self {
            repository: PostgresPlacesRepository::new(pool),
        }
    }

    pub async fn list_for_member(
        &self,
        user_id: Uuid,
        window: PageWindow,
    ) -> Result<PlacePageResponse, PlaceError> {
        self.repository
            .list_for_member(user_id, window)
            .await
            .map(|places| {
                let page = Page::from_rows(places, window);
                PlacePageResponse {
                    items: page.items.into_iter().map(Into::into).collect(),
                    next_offset: page.next_offset,
                }
            })
            .map_err(|_| PlaceError::Unavailable)
    }

    pub async fn create(
        &self,
        space_id: Uuid,
        user_id: Uuid,
        request: &CreatePlaceRequest,
    ) -> Result<PlaceResponse, PlaceError> {
        let draft = validate(request)?;
        self.repository
            .create_for_member(space_id, user_id, &draft)
            .await
            .map(Into::into)
    }

    pub async fn delete(
        &self,
        space_id: Uuid,
        place_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), PlaceError> {
        self.repository
            .delete_for_member(space_id, place_id, user_id)
            .await
    }
}

fn validate(request: &CreatePlaceRequest) -> Result<SavedPlaceDraft, PlaceError> {
    let name = request.name.trim();
    let category = request.category.trim();
    let address = request.address.trim();
    if name.is_empty()
        || name.chars().count() > 120
        || !PLACE_CATEGORIES.contains(&category)
        || address.is_empty()
        || address.chars().count() > 300
    {
        return Err(PlaceError::Invalid);
    }
    let note = request
        .note
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if note.is_some_and(|value| value.chars().count() > 2000) {
        return Err(PlaceError::Invalid);
    }
    Ok(SavedPlaceDraft {
        name: name.to_owned(),
        category: category.to_owned(),
        address: address.to_owned(),
        note: note.map(str::to_owned),
    })
}
