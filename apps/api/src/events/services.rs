use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::pagination::{Page, PageWindow};

use super::{
    models::{EventDraft, EventError},
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
            .map_err(|_| EventError::Unavailable)
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
        self.repository
            .update_for_member(event_id, user_id, &draft)
            .await
            .map(Into::into)
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
    if request
        .recurrence
        .as_deref()
        .is_some_and(|value| !["daily", "weekly", "monthly"].contains(&value))
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
