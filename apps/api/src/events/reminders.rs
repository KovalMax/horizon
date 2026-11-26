use chrono::{DateTime, Duration, NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::reminder_offsets::PRESETS;

use super::{
    recurrence,
    repositories::{EventRow, OverrideRow, PostgresEventsRepository},
};

const EVENT_BATCH_SIZE: i64 = 128;

#[derive(Clone)]
pub struct CalendarReminderWorker {
    pool: PgPool,
}

impl CalendarReminderWorker {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn run_once_at(&self, now: DateTime<Utc>) -> Result<(), sqlx::Error> {
        let mut after = None;
        while let Some(last) = self.run_batch(now, after).await? {
            after = Some(last);
        }
        Ok(())
    }

    async fn run_batch(
        &self,
        now: DateTime<Utc>,
        after: Option<Uuid>,
    ) -> Result<Option<Uuid>, sqlx::Error> {
        // Use a short catch-up window, but never send after an event has started.
        let earliest = now - Duration::minutes(10);
        let latest_start = now + Duration::weeks(1);
        // Pad the occurrence range for time zones on either side of UTC.
        let start = (now - Duration::days(1)).date_naive();
        let end = (latest_start + Duration::days(1)).date_naive();
        let mut tx = self.pool.begin().await?;
        // Keep event edits and reminder generation serialized, but release the
        // locks at each page boundary instead of retaining them for a full pass.
        let events = sqlx::query_as::<_, EventRow>(
            "SELECT e.id, e.space_id, e.title, e.starts_at, e.ends_at, e.time_zone, \
                    e.all_day, e.location, e.notes, e.recurrence, e.recurrence_ends_on, \
                    s.name AS space_name, s.kind AS space_kind, \
                    '{}'::integer[] AS my_reminder_offsets_minutes \
              FROM calendar_events e JOIN spaces s ON s.id = e.space_id \
              WHERE s.deleted_at IS NULL \
                AND e.id >= $4 AND ($5 OR e.id > $4) \
               AND EXISTS (SELECT 1 FROM calendar_reminder_preferences p WHERE p.event_id = e.id) \
               AND ((e.recurrence IS NULL AND NOT e.all_day AND e.starts_at > $1 AND e.starts_at <= $2) \
                 OR (e.recurrence IS NOT NULL AND ( \
                     (e.starts_at <= $2 AND (e.recurrence_ends_on IS NULL OR e.recurrence_ends_on >= $3)) \
                     OR EXISTS (SELECT 1 FROM calendar_event_occurrence_overrides o \
                                WHERE o.event_id = e.id AND o.starts_at > $1 AND o.starts_at <= $2)))) \
              ORDER BY e.id LIMIT $6 FOR SHARE OF e",
        )
        .bind(now)
        .bind(latest_start)
        .bind(start)
        .bind(after.unwrap_or_default())
        .bind(after.is_none())
        .bind(EVENT_BATCH_SIZE)
        .fetch_all(&mut *tx)
        .await?;
        let events: Vec<super::models::CalendarEvent> =
            events.into_iter().map(Into::into).collect();
        let Some(last) = events.last().map(|event| event.id) else {
            tx.commit().await?;
            return Ok(None);
        };
        let ids: Vec<_> = events.iter().map(|event| event.id).collect();
        let overrides = sqlx::query_as::<_, OverrideRow>(
            "SELECT event_id, occurrence_date, cancelled, title, starts_at, ends_at, \
                    time_zone, all_day, location, notes \
             FROM calendar_event_occurrence_overrides \
             WHERE event_id = ANY($1) AND (occurrence_date BETWEEN $2 AND $3 \
                   OR (starts_at > $4 AND starts_at <= $5))",
        )
        .bind(ids)
        .bind(start)
        .bind(end)
        .bind(now)
        .bind(latest_start)
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(super::models::OccurrenceOverride::from)
        .filter(|override_item| {
            events
                .binary_search_by_key(&override_item.event_id, |event| event.id)
                .is_ok_and(|index| {
                    recurrence::occurs_on(&events[index], override_item.occurrence_date)
                })
        })
        .collect();
        for occurrence in recurrence::expand(events, overrides, start, end, None) {
            if occurrence.all_day || occurrence.starts_at <= now {
                continue;
            }
            let due_offsets: Vec<_> = PRESETS
                .into_iter()
                .filter(|offset| {
                    let scheduled = occurrence.starts_at - Duration::minutes(i64::from(*offset));
                    scheduled > earliest && scheduled <= now
                })
                .collect();
            if due_offsets.is_empty() {
                continue;
            }
            sqlx::query(
                "INSERT INTO web_push_outbox \
                     (endpoint, space_id, reminder_key, kind, item_id, occurrence_date, reference_time, offset_minutes, expires_at) \
                 SELECT d.endpoint, $2, concat('calendar:', $1::uuid, ':', $3::date, ':', preset.value, ':', extract(epoch FROM $5::timestamptz)), \
                        'calendar', $1, $3, $5::timestamptz, preset.value, least($5::timestamptz, now() + interval '10 minutes') \
                 FROM calendar_reminder_preferences p \
                 JOIN web_push_subscriptions d ON d.user_id = p.user_id AND d.calendar_reminders \
                 JOIN space_memberships m ON m.space_id = $2 AND m.user_id = p.user_id \
                 CROSS JOIN LATERAL unnest(p.offsets_minutes) AS preset(value) \
                 WHERE p.event_id = $1 AND preset.value = ANY($4) \
                   AND NOT EXISTS (SELECT 1 FROM space_notification_preferences \
                     WHERE user_id = p.user_id AND space_id = $2 AND muted) \
                 ON CONFLICT (endpoint, reminder_key) DO NOTHING",
            )
            .bind(occurrence.event.id)
            .bind(occurrence.event.space_id)
            .bind(occurrence.occurrence_date)
            .bind(&due_offsets)
            .bind(occurrence.starts_at)
            .execute(&mut *tx)
            .await?;
            sqlx::query(
                "WITH delivered AS ( \
                     INSERT INTO calendar_reminder_deliveries (event_id, user_id, occurrence_date, offset_minutes) \
                     SELECT p.event_id, p.user_id, $3, preset.value \
                     FROM calendar_reminder_preferences p \
                     JOIN space_memberships m ON m.space_id = $2 AND m.user_id = p.user_id \
                     JOIN spaces s ON s.id = m.space_id AND s.deleted_at IS NULL \
                     CROSS JOIN LATERAL unnest(p.offsets_minutes) AS preset(value) \
                     WHERE p.event_id = $1 AND preset.value = ANY($4) \
                       AND NOT EXISTS (SELECT 1 FROM space_notification_preferences \
                         WHERE user_id = p.user_id AND space_id = $2 AND muted) \
                       AND COALESCE((SELECT enabled FROM notification_preferences \
                         WHERE user_id = p.user_id AND event_type = 'due_reminder' AND channel = 'in_app'), TRUE) \
                     ON CONFLICT DO NOTHING RETURNING user_id \
                 ) \
                 INSERT INTO notifications (id, recipient_id, space_id, event_type, title, body, destination) \
                 SELECT gen_random_uuid(), user_id, $2, 'due_reminder', 'Calendar reminder', $5, 'calendar' \
                 FROM delivered",
            )
            .bind(occurrence.event.id)
            .bind(occurrence.event.space_id)
            .bind(occurrence.occurrence_date)
            .bind(due_offsets)
            .bind(&occurrence.title)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(Some(last))
    }
}

pub(crate) async fn push_reminder_is_current(
    pool: &PgPool,
    user: Uuid,
    event: Uuid,
    date: Option<NaiveDate>,
    starts_at: DateTime<Utc>,
    offset: i32,
) -> Result<bool, sqlx::Error> {
    let unavailable = || sqlx::Error::Protocol("Calendar reminder lookup failed".into());
    let repository = PostgresEventsRepository::new(pool.clone());
    let Some(date) = date else { return Ok(false) };
    let Some(series) = repository
        .find_series_for_member(event, user)
        .await
        .map_err(|_| unavailable())?
    else {
        return Ok(false);
    };
    if !series.my_reminder_offsets_minutes.contains(&offset)
        || !recurrence::occurs_on(&series, date)
    {
        return Ok(false);
    }
    let overrides = repository.overrides_for_event(event, date).await?;
    Ok(
        recurrence::expand(vec![series], overrides, date, date, None)
            .into_iter()
            .any(|item| {
                !item.all_day && item.starts_at == starts_at && item.starts_at > Utc::now()
            }),
    )
}
