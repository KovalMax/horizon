use sqlx::PgPool;
use uuid::Uuid;

use crate::pagination::PageWindow;

use super::models::{PlaceError, SavedPlace, SavedPlaceDraft};

#[derive(Clone)]
pub(super) struct PostgresPlacesRepository {
    pool: PgPool,
}

impl PostgresPlacesRepository {
    pub(super) fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub(super) async fn list_for_member(
        &self,
        user_id: Uuid,
        page: PageWindow,
    ) -> Result<Vec<SavedPlace>, sqlx::Error> {
        let rows = sqlx::query_as::<_, PlaceRow>(
            "SELECT p.id, p.space_id, p.name, p.category, p.address, p.note, \
             s.name AS space_name, s.kind AS space_kind \
             FROM saved_places p \
             JOIN spaces s ON s.id = p.space_id \
             JOIN space_memberships m ON m.space_id = p.space_id AND m.user_id = $1 \
             ORDER BY p.category, p.name, p.id LIMIT $2 OFFSET $3",
        )
        .bind(user_id)
        .bind(page.limit + 1)
        .bind(page.offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub(super) async fn create_for_member(
        &self,
        space_id: Uuid,
        user_id: Uuid,
        draft: &SavedPlaceDraft,
    ) -> Result<SavedPlace, PlaceError> {
        let id = Uuid::new_v4();
        let inserted = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO saved_places (id, space_id, creator_id, name, category, address, note) \
             SELECT $1, $2, $3, $4, $5, $6, $7 \
             WHERE EXISTS (SELECT 1 FROM space_memberships \
                           WHERE space_id = $2 AND user_id = $3 AND role IN ('owner', 'editor')) \
             RETURNING id",
        )
        .bind(id)
        .bind(space_id)
        .bind(user_id)
        .bind(&draft.name)
        .bind(&draft.category)
        .bind(&draft.address)
        .bind(&draft.note)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| PlaceError::Unavailable)?;
        if inserted.is_none() {
            return Err(self.space_write_error(space_id, user_id).await?);
        }
        self.find_for_member(id, user_id)
            .await?
            .ok_or(PlaceError::NotFound)
    }

    async fn find_for_member(
        &self,
        place_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<SavedPlace>, PlaceError> {
        sqlx::query_as::<_, PlaceRow>(
            "SELECT p.id, p.space_id, p.name, p.category, p.address, p.note, \
             s.name AS space_name, s.kind AS space_kind \
             FROM saved_places p JOIN spaces s ON s.id = p.space_id \
             JOIN space_memberships m ON m.space_id = p.space_id AND m.user_id = $2 \
             WHERE p.id = $1",
        )
        .bind(place_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(Into::into))
        .map_err(|_| PlaceError::Unavailable)
    }

    async fn space_write_error(
        &self,
        space_id: Uuid,
        user_id: Uuid,
    ) -> Result<PlaceError, PlaceError> {
        let role = sqlx::query_scalar::<_, String>(
            "SELECT role FROM space_memberships WHERE space_id = $1 AND user_id = $2",
        )
        .bind(space_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| PlaceError::Unavailable)?;
        Ok(if role.as_deref() == Some("viewer") {
            PlaceError::Forbidden
        } else {
            PlaceError::NotFound
        })
    }
}

#[derive(sqlx::FromRow)]
struct PlaceRow {
    id: Uuid,
    space_id: Uuid,
    name: String,
    category: String,
    address: String,
    note: Option<String>,
    space_name: String,
    space_kind: String,
}

impl From<PlaceRow> for SavedPlace {
    fn from(row: PlaceRow) -> Self {
        Self {
            id: row.id,
            space_id: row.space_id,
            name: row.name,
            category: row.category,
            address: row.address,
            note: row.note,
            space_name: row.space_name,
            space_kind: row.space_kind,
        }
    }
}
