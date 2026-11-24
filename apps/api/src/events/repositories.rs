use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::pagination::PageWindow;

use super::models::{CalendarEvent, EventDraft, EventError, OccurrenceOverride};

#[derive(Clone)]
pub(super) struct PostgresEventsRepository {
    pool: PgPool,
}

impl PostgresEventsRepository {
    pub(super) fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub(super) async fn overrides_for_event(
        &self,
        event: Uuid,
        date: NaiveDate,
    ) -> Result<Vec<OccurrenceOverride>, sqlx::Error> {
        Ok(sqlx::query_as::<_, OverrideRow>(
            "SELECT event_id, occurrence_date, cancelled, title, starts_at, \
                    ends_at, time_zone, all_day, location, notes \
             FROM calendar_event_occurrence_overrides WHERE event_id = $1 AND occurrence_date = $2",
        )
        .bind(event)
        .bind(date)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(Into::into)
        .collect())
    }

    pub(super) async fn list_for_member(
        &self,
        user_id: Uuid,
        page: PageWindow,
    ) -> Result<Vec<CalendarEvent>, sqlx::Error> {
        let rows = sqlx::query_as::<_, EventRow>(
            "SELECT e.id, e.space_id, e.title, e.starts_at, e.ends_at, e.time_zone, \
              e.all_day, e.location, e.notes, e.recurrence, e.recurrence_ends_on, s.name AS space_name, s.kind AS space_kind, \
              COALESCE(p.offsets_minutes, '{}') AS my_reminder_offsets_minutes \
             FROM calendar_events e \
             JOIN spaces s ON s.id = e.space_id \
              JOIN space_memberships m ON m.space_id = e.space_id AND m.user_id = $1 \
              LEFT JOIN calendar_reminder_preferences p ON p.event_id = e.id AND p.user_id = m.user_id \
             ORDER BY e.starts_at, e.id LIMIT $2 OFFSET $3",
        )
        .bind(user_id)
        .bind(page.limit + 1)
        .bind(page.offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub(super) async fn list_series_for_member(
        &self,
        user_id: Uuid,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<CalendarEvent>, EventError> {
        sqlx::query_as::<_, EventRow>(
            "SELECT e.id, e.space_id, e.title, e.starts_at, e.ends_at, e.time_zone, \
             e.all_day, e.location, e.notes, e.recurrence, e.recurrence_ends_on, \
              s.name AS space_name, s.kind AS space_kind, \
              COALESCE(p.offsets_minutes, '{}') AS my_reminder_offsets_minutes \
             FROM calendar_events e JOIN spaces s ON s.id = e.space_id \
              JOIN space_memberships m ON m.space_id = e.space_id AND m.user_id = $1 \
              LEFT JOIN calendar_reminder_preferences p ON p.event_id = e.id AND p.user_id = m.user_id \
             WHERE e.starts_at < (($3::date + 1)::timestamp AT TIME ZONE e.time_zone) \
               AND ((e.recurrence IS NOT NULL AND \
                     (e.recurrence_ends_on IS NULL OR e.recurrence_ends_on >= $2)) \
                 OR (e.recurrence IS NULL AND \
                     COALESCE(e.ends_at, e.starts_at) >= ($2::date::timestamp AT TIME ZONE e.time_zone))) \
             ORDER BY e.starts_at, e.id",
        )
        .bind(user_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_all(&self.pool)
        .await
        .map(|rows| rows.into_iter().map(Into::into).collect())
        .map_err(|error| {
            log::error!(target: "horizon_api::database", "events.list_series_for_member failed: {error}");
            EventError::Unavailable
        })
    }

    pub(super) async fn overrides_for_member(
        &self,
        user_id: Uuid,
        start_date: NaiveDate,
        end_date: NaiveDate,
        start_instant: DateTime<Utc>,
        end_instant: DateTime<Utc>,
    ) -> Result<Vec<OccurrenceOverride>, EventError> {
        sqlx::query_as::<_, OverrideRow>(
            "SELECT o.event_id, o.occurrence_date, o.cancelled, o.title, o.starts_at, \
             o.ends_at, o.time_zone, o.all_day, o.location, o.notes \
             FROM calendar_event_occurrence_overrides o \
             JOIN calendar_events e ON e.id = o.event_id \
             JOIN space_memberships m ON m.space_id = e.space_id AND m.user_id = $1 \
             WHERE o.occurrence_date BETWEEN $2 AND $3 \
                OR (o.starts_at >= $4 AND o.starts_at < $5)",
        )
        .bind(user_id)
        .bind(start_date)
        .bind(end_date)
        .bind(start_instant)
        .bind(end_instant)
        .fetch_all(&self.pool)
        .await
        .map(|rows| rows.into_iter().map(Into::into).collect())
        .map_err(|error| {
            log::error!(target: "horizon_api::database", "events.overrides_for_member failed: {error}");
            EventError::Unavailable
        })
    }

    pub(super) async fn set_occurrence_override(
        &self,
        event_id: Uuid,
        user_id: Uuid,
        occurrence_date: NaiveDate,
        draft: &EventDraft,
    ) -> Result<(), EventError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| EventError::Unavailable)?;
        let context = sqlx::query_as::<_, (Uuid, String, String)>(
            "SELECT e.space_id, s.name, e.title FROM calendar_events e \
             JOIN spaces s ON s.id = e.space_id \
             JOIN space_memberships m ON m.space_id = e.space_id AND m.user_id = $2 \
             WHERE e.id = $1 AND e.recurrence IS NOT NULL AND m.role IN ('owner', 'editor') \
             FOR UPDATE OF e",
        )
        .bind(event_id)
        .bind(user_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| EventError::Unavailable)?;
        let Some((space_id, space_name, _)) = context else {
            transaction
                .rollback()
                .await
                .map_err(|_| EventError::Unavailable)?;
            return Err(self.event_access_error(event_id, user_id).await?);
        };
        let updated = sqlx::query(
            "INSERT INTO calendar_event_occurrence_overrides \
             (event_id, occurrence_date, title, starts_at, ends_at, time_zone, all_day, location, notes) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
             ON CONFLICT (event_id, occurrence_date) DO UPDATE SET cancelled = FALSE, \
             title = EXCLUDED.title, starts_at = EXCLUDED.starts_at, ends_at = EXCLUDED.ends_at, \
             time_zone = EXCLUDED.time_zone, all_day = EXCLUDED.all_day, \
             location = EXCLUDED.location, notes = EXCLUDED.notes, updated_at = now() \
             WHERE ROW(calendar_event_occurrence_overrides.cancelled, calendar_event_occurrence_overrides.title, \
                       calendar_event_occurrence_overrides.starts_at, calendar_event_occurrence_overrides.ends_at, \
                       calendar_event_occurrence_overrides.time_zone, calendar_event_occurrence_overrides.all_day, \
                       calendar_event_occurrence_overrides.location, calendar_event_occurrence_overrides.notes) \
                   IS DISTINCT FROM ROW(EXCLUDED.cancelled, EXCLUDED.title, EXCLUDED.starts_at, EXCLUDED.ends_at, \
                                        EXCLUDED.time_zone, EXCLUDED.all_day, EXCLUDED.location, EXCLUDED.notes)",
        )
        .bind(event_id).bind(occurrence_date).bind(&draft.title).bind(draft.starts_at)
        .bind(draft.ends_at).bind(&draft.time_zone).bind(draft.all_day)
        .bind(&draft.location).bind(&draft.notes)
        .execute(&mut *transaction).await.map_err(|_| EventError::Unavailable)?;
        if updated.rows_affected() > 0 {
            crate::notifications::enqueue_space_members(
                &mut transaction,
                space_id,
                user_id,
                &crate::notifications::NotificationContent {
                    event_type: "event_change",
                    title: "A calendar event changed",
                    body: &format!("{} · {}", space_name, draft.title),
                    destination: "calendar",
                },
            )
            .await
            .map_err(|_| EventError::Unavailable)?;
        }
        transaction
            .commit()
            .await
            .map_err(|_| EventError::Unavailable)?;
        Ok(())
    }

    pub(super) async fn find_series_for_member(
        &self,
        event_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<CalendarEvent>, EventError> {
        self.find_for_member(event_id, user_id).await
    }

    pub(super) async fn prune_invalid_overrides(
        &self,
        event: &CalendarEvent,
    ) -> Result<(), EventError> {
        let dates = sqlx::query_scalar::<_, NaiveDate>(
            "SELECT occurrence_date FROM calendar_event_occurrence_overrides WHERE event_id = $1",
        )
        .bind(event.id)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| EventError::Unavailable)?;
        for date in dates {
            if !super::recurrence::occurs_on(event, date) {
                sqlx::query("DELETE FROM calendar_event_occurrence_overrides WHERE event_id = $1 AND occurrence_date = $2")
                    .bind(event.id)
                    .bind(date)
                    .execute(&self.pool)
                    .await
                    .map_err(|_| EventError::Unavailable)?;
            }
        }
        Ok(())
    }

    pub(super) async fn cancel_occurrence(
        &self,
        event_id: Uuid,
        user_id: Uuid,
        occurrence_date: NaiveDate,
    ) -> Result<(), EventError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| EventError::Unavailable)?;
        let context = sqlx::query_as::<_, (Uuid, String, String)>(
            "SELECT e.space_id, s.name, e.title FROM calendar_events e \
             JOIN spaces s ON s.id = e.space_id \
             JOIN space_memberships m ON m.space_id = e.space_id AND m.user_id = $2 \
             WHERE e.id = $1 AND e.recurrence IS NOT NULL AND m.role IN ('owner', 'editor') \
             FOR UPDATE OF e",
        )
        .bind(event_id)
        .bind(user_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| EventError::Unavailable)?;
        let Some((space_id, space_name, title)) = context else {
            transaction
                .rollback()
                .await
                .map_err(|_| EventError::Unavailable)?;
            return Err(self.event_access_error(event_id, user_id).await?);
        };
        let cancelled = sqlx::query(
            "INSERT INTO calendar_event_occurrence_overrides (event_id, occurrence_date, cancelled) \
             VALUES ($1, $2, TRUE) ON CONFLICT (event_id, occurrence_date) DO UPDATE SET \
             cancelled = TRUE, title = NULL, starts_at = NULL, ends_at = NULL, time_zone = NULL, \
             all_day = NULL, location = NULL, notes = NULL, updated_at = now() \
             WHERE NOT calendar_event_occurrence_overrides.cancelled",
        )
        .bind(event_id).bind(occurrence_date).execute(&mut *transaction).await
        .map_err(|_| EventError::Unavailable)?;
        if cancelled.rows_affected() > 0 {
            crate::notifications::enqueue_space_members(
                &mut transaction,
                space_id,
                user_id,
                &crate::notifications::NotificationContent {
                    event_type: "event_change",
                    title: "A calendar event occurrence was cancelled",
                    body: &format!("{} · {} · {}", space_name, title, occurrence_date),
                    destination: "calendar",
                },
            )
            .await
            .map_err(|_| EventError::Unavailable)?;
        }
        transaction
            .commit()
            .await
            .map_err(|_| EventError::Unavailable)?;
        Ok(())
    }

    pub(super) async fn create_for_member(
        &self,
        space_id: Uuid,
        user_id: Uuid,
        draft: &EventDraft,
    ) -> Result<CalendarEvent, EventError> {
        let id = Uuid::new_v4();
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| EventError::Unavailable)?;
        let inserted = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO calendar_events \
             (id, space_id, creator_id, title, starts_at, ends_at, time_zone, all_day, location, notes, recurrence, recurrence_ends_on) \
             SELECT $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12 \
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
        .bind(draft.recurrence_ends_on)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| EventError::Unavailable)?;

        if inserted.is_none() {
            return Err(self.membership_error(space_id, user_id).await?);
        }
        if let Some(offsets) = &draft.my_reminder_offsets_minutes {
            save_reminders(&mut transaction, id, user_id, offsets).await?;
        }
        let space_name = sqlx::query_scalar::<_, String>("SELECT name FROM spaces WHERE id = $1")
            .bind(space_id)
            .fetch_one(&mut *transaction)
            .await
            .map_err(|_| EventError::Unavailable)?;
        crate::notifications::enqueue_space_members(
            &mut transaction,
            space_id,
            user_id,
            &crate::notifications::NotificationContent {
                event_type: "event_change",
                title: "A calendar event was added",
                body: &format!("{} · {}", space_name, draft.title),
                destination: "calendar",
            },
        )
        .await
        .map_err(|_| EventError::Unavailable)?;
        transaction
            .commit()
            .await
            .map_err(|_| EventError::Unavailable)?;
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
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| EventError::Unavailable)?;
        let space_id = sqlx::query_scalar::<_, Uuid>(
            "SELECT e.space_id FROM calendar_events e JOIN space_memberships m \
             ON m.space_id = e.space_id AND m.user_id = $2 AND m.role IN ('owner', 'editor') \
             WHERE e.id = $1 FOR UPDATE OF e",
        )
        .bind(event_id)
        .bind(user_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| EventError::Unavailable)?;
        let Some(space_id) = space_id else {
            return Err(self.event_access_error(event_id, user_id).await?);
        };
        if let Some(offsets) = &draft.my_reminder_offsets_minutes {
            save_reminders(&mut transaction, event_id, user_id, offsets).await?;
        }
        let updated = sqlx::query_scalar::<_, Uuid>(
            "UPDATE calendar_events e SET title = $3, starts_at = $4, ends_at = $5, \
             time_zone = $6, all_day = $7, location = $8, notes = $9, recurrence = $10, recurrence_ends_on = $11, updated_at = now() \
             WHERE e.id = $1 AND EXISTS (SELECT 1 FROM space_memberships m \
                                         WHERE m.space_id = e.space_id AND m.user_id = $2 \
                                           AND m.role IN ('owner', 'editor')) \
               AND ROW(e.title, e.starts_at, e.ends_at, e.time_zone, e.all_day, e.location, e.notes, e.recurrence, e.recurrence_ends_on) \
                   IS DISTINCT FROM ROW($3, $4, $5, $6, $7, $8, $9, $10, $11) \
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
        .bind(draft.recurrence_ends_on)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| EventError::Unavailable)?;

        if updated.is_some() {
            let space_name =
                sqlx::query_scalar::<_, String>("SELECT name FROM spaces WHERE id = $1")
                    .bind(space_id)
                    .fetch_one(&mut *transaction)
                    .await
                    .map_err(|_| EventError::Unavailable)?;
            crate::notifications::enqueue_space_members(
                &mut transaction,
                space_id,
                user_id,
                &crate::notifications::NotificationContent {
                    event_type: "event_change",
                    title: "A calendar event changed",
                    body: &format!("{} · {}", space_name, draft.title),
                    destination: "calendar",
                },
            )
            .await
            .map_err(|_| EventError::Unavailable)?;
        }
        transaction
            .commit()
            .await
            .map_err(|_| EventError::Unavailable)?;
        self.find_for_member(event_id, user_id)
            .await?
            .ok_or(EventError::NotFound)
    }

    pub(super) async fn delete_for_member(
        &self,
        event_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), EventError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| EventError::Unavailable)?;
        let event = sqlx::query_as::<_, (Uuid, String, String)>(
            "SELECT e.space_id, e.title, s.name FROM calendar_events e \
             JOIN spaces s ON s.id = e.space_id \
             JOIN space_memberships m ON m.space_id = e.space_id AND m.user_id = $2 \
             WHERE e.id = $1 AND m.role IN ('owner', 'editor') FOR UPDATE OF e",
        )
        .bind(event_id)
        .bind(user_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| EventError::Unavailable)?;
        let Some((space_id, title, space_name)) = event else {
            return Err(self.event_access_error(event_id, user_id).await?);
        };
        crate::notifications::enqueue_space_members(
            &mut transaction,
            space_id,
            user_id,
            &crate::notifications::NotificationContent {
                event_type: "event_change",
                title: "A calendar event was removed",
                body: &format!("{} · {}", space_name, title),
                destination: "calendar",
            },
        )
        .await
        .map_err(|_| EventError::Unavailable)?;
        let deleted = sqlx::query_scalar::<_, Uuid>(
            "DELETE FROM calendar_events e \
             WHERE e.id = $1 AND EXISTS (SELECT 1 FROM space_memberships m \
                                        WHERE m.space_id = e.space_id AND m.user_id = $2 \
                                          AND m.role IN ('owner', 'editor')) \
             RETURNING e.id",
        )
        .bind(event_id)
        .bind(user_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| EventError::Unavailable)?;

        if deleted.is_some() {
            transaction
                .commit()
                .await
                .map_err(|_| EventError::Unavailable)?;
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
              e.all_day, e.location, e.notes, e.recurrence, e.recurrence_ends_on, s.name AS space_name, s.kind AS space_kind, \
              COALESCE(p.offsets_minutes, '{}') AS my_reminder_offsets_minutes \
             FROM calendar_events e \
             JOIN spaces s ON s.id = e.space_id \
              JOIN space_memberships m ON m.space_id = e.space_id AND m.user_id = $2 \
              LEFT JOIN calendar_reminder_preferences p ON p.event_id = e.id AND p.user_id = m.user_id \
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

async fn save_reminders(
    transaction: &mut Transaction<'_, Postgres>,
    event_id: Uuid,
    user_id: Uuid,
    offsets: &[i32],
) -> Result<(), EventError> {
    if offsets.is_empty() {
        sqlx::query(
            "DELETE FROM calendar_reminder_preferences WHERE event_id = $1 AND user_id = $2",
        )
        .bind(event_id)
        .bind(user_id)
        .execute(&mut **transaction)
        .await
    } else {
        sqlx::query(
            "INSERT INTO calendar_reminder_preferences (event_id, user_id, offsets_minutes) \
             VALUES ($1, $2, $3) ON CONFLICT (event_id, user_id) DO UPDATE \
             SET offsets_minutes = EXCLUDED.offsets_minutes",
        )
        .bind(event_id)
        .bind(user_id)
        .bind(offsets)
        .execute(&mut **transaction)
        .await
    }
    .map(|_| ())
    .map_err(|_| EventError::Unavailable)
}

#[derive(sqlx::FromRow)]
pub(super) struct EventRow {
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
    recurrence_ends_on: Option<NaiveDate>,
    space_name: String,
    space_kind: String,
    my_reminder_offsets_minutes: Vec<i32>,
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
            recurrence_ends_on: row.recurrence_ends_on,
            space_name: row.space_name,
            space_kind: row.space_kind,
            my_reminder_offsets_minutes: row.my_reminder_offsets_minutes,
        }
    }
}

#[derive(sqlx::FromRow)]
pub(super) struct OverrideRow {
    event_id: Uuid,
    occurrence_date: NaiveDate,
    cancelled: bool,
    title: Option<String>,
    starts_at: Option<DateTime<Utc>>,
    ends_at: Option<DateTime<Utc>>,
    time_zone: Option<String>,
    all_day: Option<bool>,
    location: Option<String>,
    notes: Option<String>,
}

impl From<OverrideRow> for OccurrenceOverride {
    fn from(row: OverrideRow) -> Self {
        Self {
            event_id: row.event_id,
            occurrence_date: row.occurrence_date,
            cancelled: row.cancelled,
            title: row.title,
            starts_at: row.starts_at,
            ends_at: row.ends_at,
            time_zone: row.time_zone,
            all_day: row.all_day,
            location: row.location,
            notes: row.notes,
        }
    }
}
