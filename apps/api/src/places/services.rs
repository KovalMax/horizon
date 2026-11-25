use sqlx::PgPool;
use uuid::Uuid;

use crate::pagination::{Page, PageWindow};

use super::{
    maps_import::{ImportError, MapsImport, PlaceImportPreview},
    models::{PlaceError, SavedPlaceDraft},
    repositories::PostgresPlacesRepository,
    requests::CreatePlaceRequest,
    responses::{PlacePageResponse, PlaceResponse},
};

const PLACE_CATEGORIES: [&str; 4] = ["Food & drink", "Groceries", "Health", "Services"];

#[derive(Clone)]
pub struct PlacesApplication {
    repository: PostgresPlacesRepository,
    maps_import: MapsImport,
}

pub(super) enum ImportPreviewError {
    Access(PlaceError),
    Import(ImportError),
}

impl PlacesApplication {
    pub fn new(pool: PgPool) -> Self {
        Self {
            repository: PostgresPlacesRepository::new(pool),
            maps_import: MapsImport::default(),
        }
    }

    pub(super) async fn import_preview(
        &self,
        space_id: Uuid,
        user_id: Uuid,
        url: &str,
    ) -> Result<PlaceImportPreview, ImportPreviewError> {
        self.repository
            .require_space_writer(space_id, user_id)
            .await
            .map_err(ImportPreviewError::Access)?;
        self.maps_import
            .resolve(user_id, url)
            .await
            .map_err(ImportPreviewError::Import)
    }

    pub async fn list_for_member(
        &self,
        user_id: Uuid,
        window: PageWindow,
        space_id: Option<Uuid>,
        category: Option<&str>,
    ) -> Result<PlacePageResponse, PlaceError> {
        if category.is_some_and(|value| !PLACE_CATEGORIES.contains(&value)) {
            return Err(PlaceError::Invalid);
        }
        let rows = self
            .repository
            .list_for_member(user_id, window, space_id, category)
            .await
            .map_err(|_| PlaceError::Unavailable)?;
        let category_counts = match space_id {
            Some(space_id) => Some(
                self.repository
                    .category_counts_for_member(space_id, user_id)
                    .await
                    .map_err(|_| PlaceError::Unavailable)?,
            ),
            None => None,
        };
        let page = Page::from_rows(rows, window);
        Ok(PlacePageResponse {
            items: page.items.into_iter().map(Into::into).collect(),
            next_offset: page.next_offset,
            category_counts,
        })
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
