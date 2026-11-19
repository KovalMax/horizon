use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::pagination::{Page, PageWindow};

use super::{
    models::{EventDraft, EventError},
    recurrence,
    repositories::PostgresEventsRepository,
    requests::CreateEventRequest,
    responses::{EventPageResponse, EventResponse},
};

#[derive(Clone)]
pub struct EventsApplication {
    repository: PostgresEventsRepository,
}

impl EventsApplication {
    pub fn new(pool: PgPool) -> Self {
        Self {
            repository: PostgresEventsRepository::new(pool),
        }
    }

    pub async fn list_for_member(
        &self,
        user_id: Uuid,
        page: PageWindow,
    ) -> Result<EventPageResponse, EventError> {
        self.repository
            .list_for_member(user_id, page)
            .await
            .map(|events| {
                let page = Page::from_rows(events, page);
                EventPageResponse {
                    items: page.items.into_iter().map(Into::into).collect(),
                    next_offset: page.next_offset,
                }
            })
            .map_err(|error| {
                log::error!(target: "horizon_api::database", "events.list_for_member failed: {error}");
                EventError::Unavailable
            })
    }

    pub async fn list_occurrences_for_member(
        &self,
        user_id: Uuid,
        start_date: NaiveDate,
        end_date: NaiveDate,
        page: PageWindow,
    ) -> Result<EventPageResponse, EventError> {
        if end_date < start_date || end_date.signed_duration_since(start_date).num_days() > 90 {
            return Err(EventError::Invalid);
        }
        let events = self
            .repository
            .list_series_for_member(user_id, start_date, end_date)
            .await?;
        let overrides = self
            .repository
            .overrides_for_member(
                user_id,
                start_date,
                end_date,
                start_date.and_hms_opt(0, 0, 0).unwrap().and_utc(),
                end_date
                    .succ_opt()
                    .unwrap_or(end_date)
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc(),
            )
            .await?;
        let occurrences = recurrence::expand(events, overrides, start_date, end_date)
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>();
        let page = Page::from_rows(occurrences, page);
        Ok(EventPageResponse {
            items: page.items,
            next_offset: page.next_offset,
        })
    }

    pub async fn create(
        &self,
        space_id: Uuid,
        user_id: Uuid,
        request: &CreateEventRequest,
    ) -> Result<EventResponse, EventError> {
        let draft = validate(request)?;
        self.repository
            .create_for_member(space_id, user_id, &draft)
            .await
            .map(Into::into)
    }

    pub async fn update(
        &self,
        event_id: Uuid,
        user_id: Uuid,
        request: &CreateEventRequest,
    ) -> Result<EventResponse, EventError> {
        let draft = validate(request)?;
        let updated = self
            .repository
            .update_for_member(event_id, user_id, &draft)
            .await?;
        self.repository.prune_invalid_overrides(&updated).await?;
        Ok(updated.into())
    }

    pub async fn update_occurrence(
        &self,
        event_id: Uuid,
        user_id: Uuid,
        occurrence_date: NaiveDate,
        request: &CreateEventRequest,
    ) -> Result<EventResponse, EventError> {
        let draft = validate(request)?;
        if draft.recurrence.is_some() || draft.recurrence_ends_on.is_some() {
            return Err(EventError::Invalid);
        }
        let series = self
            .repository
            .find_series_for_member(event_id, user_id)
            .await?
            .ok_or(EventError::NotFound)?;
        if series.recurrence.is_none() || !recurrence::occurs_on(&series, occurrence_date) {
            return Err(EventError::InvalidOccurrence);
        }
        self.repository
            .set_occurrence_override(event_id, user_id, occurrence_date, &draft)
            .await?;
        let mut response = EventResponse::from(series);
        response.title = draft.title;
        response.starts_at = draft.starts_at.to_rfc3339();
        response.ends_at = draft.ends_at.map(|value| value.to_rfc3339());
        response.time_zone = draft.time_zone;
        response.all_day = draft.all_day;
        response.location = draft.location;
        response.notes = draft.notes;
        response.occurrence_date = Some(occurrence_date.to_string());
        response.occurrence_id = Some(format!("{event_id}:{occurrence_date}"));
        response.is_override = true;
        Ok(response)
    }

    pub async fn cancel_occurrence(
        &self,
        event_id: Uuid,
        user_id: Uuid,
        occurrence_date: NaiveDate,
    ) -> Result<(), EventError> {
        let series = self
            .repository
            .find_series_for_member(event_id, user_id)
            .await?
            .ok_or(EventError::NotFound)?;
        if series.recurrence.is_none() || !recurrence::occurs_on(&series, occurrence_date) {
            return Err(EventError::InvalidOccurrence);
        }
        self.repository
            .cancel_occurrence(event_id, user_id, occurrence_date)
            .await
    }

    pub async fn delete(&self, event_id: Uuid, user_id: Uuid) -> Result<(), EventError> {
        self.repository.delete_for_member(event_id, user_id).await
    }
}

fn validate(request: &CreateEventRequest) -> Result<EventDraft, EventError> {
    let title = request.title.trim();
    if title.is_empty() || title.chars().count() > 120 {
        return Err(EventError::Invalid);
    }
    let starts_at = DateTime::parse_from_rfc3339(&request.starts_at)
        .map_err(|_| EventError::Invalid)?
        .with_timezone(&Utc);
    let ends_at = request
        .ends_at
        .as_deref()
        .map(|value| {
            DateTime::parse_from_rfc3339(value)
                .map(|time| time.with_timezone(&Utc))
                .map_err(|_| EventError::Invalid)
        })
        .transpose()?;
    if ends_at.is_some_and(|end| end <= starts_at) {
        return Err(EventError::Invalid);
    }

    let time_zone = request.time_zone.trim();
    if time_zone.is_empty()
        || time_zone.len() > 64
        || !time_zone
            .bytes()
            .all(|character| character.is_ascii_alphanumeric() || b"/_+-".contains(&character))
    {
        return Err(EventError::Invalid);
    }
    let zone = time_zone
        .parse::<chrono_tz::Tz>()
        .map_err(|_| EventError::Invalid)?;
    if request
        .recurrence
        .as_deref()
        .is_some_and(|value| !["daily", "weekly", "monthly"].contains(&value))
    {
        return Err(EventError::Invalid);
    }
    let recurrence_ends_on = request
        .recurrence_ends_on
        .as_deref()
        .map(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| EventError::Invalid))
        .transpose()?;
    let local_start_date = starts_at.with_timezone(&zone).date_naive();
    if (request.recurrence.is_none() && recurrence_ends_on.is_some())
        || recurrence_ends_on.is_some_and(|date| date < local_start_date)
    {
        return Err(EventError::Invalid);
    }

    Ok(EventDraft {
        title: title.to_owned(),
        starts_at,
        ends_at,
        time_zone: time_zone.to_owned(),
        all_day: request.all_day,
        location: optional_text(request.location.as_deref(), 240)?,
        notes: optional_text(request.notes.as_deref(), 2000)?,
        recurrence: request.recurrence.clone(),
        recurrence_ends_on,
    })
}

fn optional_text(value: Option<&str>, max_length: usize) -> Result<Option<String>, EventError> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if value.chars().count() > max_length {
        return Err(EventError::Invalid);
    }
    Ok(Some(value.to_owned()))
}
