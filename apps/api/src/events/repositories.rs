use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::pagination::PageWindow;

use super::models::{CalendarEvent, EventDraft, EventError};

#[derive(Clone)]
pub(super) struct PostgresEventsRepository {
    pool: PgPool,
}

impl PostgresEventsRepository {
    pub(super) fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub(super) async fn list_for_member(
        &self,
        user_id: Uuid,
        page: PageWindow,
    ) -> Result<Vec<CalendarEvent>, sqlx::Error> {
        let rows = sqlx::query_as::<_, EventRow>(
            "SELECT e.id, e.space_id, e.title, e.starts_at, e.ends_at, e.time_zone, \
             e.all_day, e.location, e.notes, e.recurrence, s.name AS space_name, s.kind AS space_kind \
             FROM calendar_events e \
             JOIN spaces s ON s.id = e.space_id \
             JOIN space_memberships m ON m.space_id = e.space_id AND m.user_id = $1 \
             ORDER BY e.starts_at, e.id LIMIT $2 OFFSET $3",
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
        draft: &EventDraft,
    ) -> Result<CalendarEvent, EventError> {
        let id = Uuid::new_v4();
        let inserted = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO calendar_events \
             (id, space_id, creator_id, title, starts_at, ends_at, time_zone, all_day, location, notes, recurrence) \
             SELECT $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11 \
             WHERE EXISTS (SELECT 1 FROM space_memberships \
                           WHERE space_id = $2 AND user_id = $3 AND role IN ('owner', 'editor')) \
             RETURNING id",
        )
        .bind(id)
        .bind(space_id)
        .bind(user_id)
        .bind(&draft.title)
        .bind(draft.starts_at)
        .bind(draft.ends_at)
        .bind(&draft.time_zone)
        .bind(draft.all_day)
        .bind(&draft.location)
        .bind(&draft.notes)
        .bind(&draft.recurrence)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| EventError::Unavailable)?;

        if inserted.is_none() {
            return Err(self.membership_error(space_id, user_id).await?);
        }
        self.find_for_member(id, user_id)
            .await?
            .ok_or(EventError::NotFound)
    }

    pub(super) async fn update_for_member(
        &self,
        event_id: Uuid,
        user_id: Uuid,
        draft: &EventDraft,
    ) -> Result<CalendarEvent, EventError> {
        let updated = sqlx::query_scalar::<_, Uuid>(
            "UPDATE calendar_events e SET title = $3, starts_at = $4, ends_at = $5, \
             time_zone = $6, all_day = $7, location = $8, notes = $9, recurrence = $10, updated_at = now() \
             WHERE e.id = $1 AND EXISTS (SELECT 1 FROM space_memberships m \
                                        WHERE m.space_id = e.space_id AND m.user_id = $2 \
                                          AND m.role IN ('owner', 'editor')) \
             RETURNING e.id",
        )
        .bind(event_id)
        .bind(user_id)
        .bind(&draft.title)
        .bind(draft.starts_at)
        .bind(draft.ends_at)
        .bind(&draft.time_zone)
        .bind(draft.all_day)
        .bind(&draft.location)
        .bind(&draft.notes)
        .bind(&draft.recurrence)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| EventError::Unavailable)?;

        if updated.is_none() {
            return Err(self.event_access_error(event_id, user_id).await?);
        }
        self.find_for_member(event_id, user_id)
            .await?
            .ok_or(EventError::NotFound)
    }

    pub(super) async fn delete_for_member(
        &self,
        event_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), EventError> {
        let deleted = sqlx::query_scalar::<_, Uuid>(
            "DELETE FROM calendar_events e \
             WHERE e.id = $1 AND EXISTS (SELECT 1 FROM space_memberships m \
                                        WHERE m.space_id = e.space_id AND m.user_id = $2 \
                                          AND m.role IN ('owner', 'editor')) \
             RETURNING e.id",
        )
        .bind(event_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| EventError::Unavailable)?;

        if deleted.is_some() {
            Ok(())
        } else {
            Err(self.event_access_error(event_id, user_id).await?)
        }
    }

    async fn find_for_member(
        &self,
        event_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<CalendarEvent>, EventError> {
        sqlx::query_as::<_, EventRow>(
            "SELECT e.id, e.space_id, e.title, e.starts_at, e.ends_at, e.time_zone, \
             e.all_day, e.location, e.notes, e.recurrence, s.name AS space_name, s.kind AS space_kind \
             FROM calendar_events e \
             JOIN spaces s ON s.id = e.space_id \
             JOIN space_memberships m ON m.space_id = e.space_id AND m.user_id = $2 \
             WHERE e.id = $1",
        )
        .bind(event_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(Into::into))
        .map_err(|_| EventError::Unavailable)
    }

    async fn membership_error(
        &self,
        space_id: Uuid,
        user_id: Uuid,
    ) -> Result<EventError, EventError> {
        let role = sqlx::query_scalar::<_, String>(
            "SELECT role FROM space_memberships WHERE space_id = $1 AND user_id = $2",
        )
        .bind(space_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| EventError::Unavailable)?;
        Ok(match role.as_deref() {
            Some("viewer") => EventError::Forbidden,
            _ => EventError::NotFound,
        })
    }

    async fn event_access_error(
        &self,
        event_id: Uuid,
        user_id: Uuid,
    ) -> Result<EventError, EventError> {
        let role = sqlx::query_scalar::<_, String>(
            "SELECT m.role FROM calendar_events e \
             JOIN space_memberships m ON m.space_id = e.space_id AND m.user_id = $2 \
             WHERE e.id = $1",
        )
        .bind(event_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| EventError::Unavailable)?;
        Ok(match role.as_deref() {
            Some("viewer") => EventError::Forbidden,
            _ => EventError::NotFound,
        })
    }
}

#[derive(sqlx::FromRow)]
struct EventRow {
    id: Uuid,
    space_id: Uuid,
    title: String,
    starts_at: DateTime<Utc>,
    ends_at: Option<DateTime<Utc>>,
    time_zone: String,
    all_day: bool,
    location: Option<String>,
    notes: Option<String>,
    recurrence: Option<String>,
    space_name: String,
    space_kind: String,
}

impl From<EventRow> for CalendarEvent {
    fn from(row: EventRow) -> Self {
        Self {
            id: row.id,
            space_id: row.space_id,
            title: row.title,
            starts_at: row.starts_at,
            ends_at: row.ends_at,
            time_zone: row.time_zone,
            all_day: row.all_day,
            location: row.location,
            notes: row.notes,
            recurrence: row.recurrence,
            space_name: row.space_name,
            space_kind: row.space_kind,
        }
    }
}
