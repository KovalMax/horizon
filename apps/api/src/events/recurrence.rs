use std::{collections::HashMap, time::Instant};

use chrono::{
    DateTime, Datelike, Duration, LocalResult, NaiveDate, NaiveDateTime, NaiveTime, Offset,
    TimeZone, Utc,
};
use chrono_tz::Tz;

use super::models::{CalendarEvent, EventOccurrence, OccurrenceOverride};

#[derive(Default)]
pub(super) struct ExpansionTimings {
    pub build_us: u128,
    pub moved_overrides_us: u128,
    pub sort_us: u128,
}

pub(super) fn expand(
    events: Vec<CalendarEvent>,
    overrides: Vec<OccurrenceOverride>,
    start: NaiveDate,
    end: NaiveDate,
    mut timings: Option<&mut ExpansionTimings>,
) -> Vec<EventOccurrence> {
    let start_build = timings.is_some().then(Instant::now);
    let event_by_id: HashMap<_, _> = events.iter().map(|event| (event.id, event)).collect();
    let mut overrides_by_key: HashMap<_, _> = overrides
        .iter()
        .map(|item| ((item.event_id, item.occurrence_date), item))
        .collect();
    let mut result = Vec::new();
    for event in &events {
        for date in occurrence_dates(event, start, end) {
            if let Some(override_item) = overrides_by_key.remove(&(event.id, date)) {
                if !override_item.cancelled {
                    if let (Some(title), Some(starts_at), Some(time_zone), Some(all_day)) = (
                        &override_item.title,
                        override_item.starts_at,
                        &override_item.time_zone,
                        override_item.all_day,
                    ) {
                        result.push(EventOccurrence {
                            event: (*event).clone(),
                            occurrence_date: date,
                            starts_at,
                            ends_at: override_item.ends_at,
                            title: title.clone(),
                            time_zone: time_zone.clone(),
                            all_day,
                            location: override_item.location.clone(),
                            notes: override_item.notes.clone(),
                            is_override: true,
                        });
                    }
                }
                continue;
            }
            if let Some(occurrence) = occurrence(event, date) {
                result.push(occurrence);
            }
        }
    }
    let start_overrides = start_build.map(|start| {
        let now = Instant::now();
        if let Some(timings) = timings.as_mut() {
            timings.build_us = (now - start).as_micros();
        }
        now
    });
    // Only overrides not consumed by an in-range scheduled date can move in
    // from outside the visible range.
    for item in overrides_by_key.into_values() {
        if item.cancelled {
            continue;
        }
        let Some(event) = event_by_id.get(&item.event_id) else {
            continue;
        };
        let (Some(title), Some(starts_at), Some(time_zone), Some(all_day)) =
            (&item.title, item.starts_at, &item.time_zone, item.all_day)
        else {
            continue;
        };
        let tz = match time_zone.parse::<Tz>() {
            Ok(tz) => tz,
            Err(_) => continue,
        };
        let actual_date = starts_at.with_timezone(&tz).date_naive();
        if actual_date < start || actual_date > end {
            continue;
        }
        result.push(EventOccurrence {
            event: (*event).clone(),
            occurrence_date: item.occurrence_date,
            starts_at,
            ends_at: item.ends_at,
            title: title.clone(),
            time_zone: time_zone.clone(),
            all_day,
            location: item.location.clone(),
            notes: item.notes.clone(),
            is_override: true,
        });
    }
    let start_sort = start_overrides.map(|start| {
        let now = Instant::now();
        if let Some(timings) = timings.as_mut() {
            timings.moved_overrides_us = (now - start).as_micros();
        }
        now
    });
    result.sort_by_key(|item| (item.starts_at, item.event.id, item.occurrence_date));
    if let (Some(start), Some(timings)) = (start_sort, timings) {
        timings.sort_us = start.elapsed().as_micros();
    }
    result
}

pub(super) fn occurs_on(event: &CalendarEvent, date: NaiveDate) -> bool {
    occurrence_dates(event, date, date).contains(&date)
}

fn occurrence_dates(event: &CalendarEvent, start: NaiveDate, end: NaiveDate) -> Vec<NaiveDate> {
    let Ok(tz) = event.time_zone.parse::<Tz>() else {
        return Vec::new();
    };
    let first = event.starts_at.with_timezone(&tz).date_naive();
    if first > end || event.recurrence_ends_on.is_some_and(|last| last < start) {
        return Vec::new();
    }
    let last = event.recurrence_ends_on.unwrap_or(end).min(end);
    let from = first.max(start);
    if from > last {
        return Vec::new();
    }
    if event.recurrence.is_none() {
        return (first >= start && first <= end)
            .then_some(first)
            .into_iter()
            .collect();
    }
    let mut date = from;
    let mut result = Vec::new();
    while date <= last {
        let matches = match event.recurrence.as_deref() {
            Some("daily") => true,
            Some("weekly") => date.weekday() == first.weekday(),
            Some("monthly") => date.day() == first.day(),
            _ => false,
        };
        if matches {
            result.push(date);
        }
        let Some(next) = date.succ_opt() else { break };
        date = next;
    }
    result
}

fn occurrence(event: &CalendarEvent, date: NaiveDate) -> Option<EventOccurrence> {
    let tz = event.time_zone.parse::<Tz>().ok()?;
    let starts = event.starts_at.with_timezone(&tz).naive_local();
    let start_local = NaiveDateTime::new(
        date,
        if event.all_day {
            NaiveTime::MIN
        } else {
            starts.time()
        },
    );
    let starts_at = resolve_local(tz, start_local)?;
    let ends_at = event.ends_at.and_then(|end| {
        if event.all_day {
            return None;
        }
        let original_end = end.with_timezone(&tz).naive_local();
        let day_offset = original_end
            .date()
            .signed_duration_since(starts.date())
            .num_days();
        date.checked_add_signed(Duration::days(day_offset))
            .and_then(|end_date| {
                resolve_local(tz, NaiveDateTime::new(end_date, original_end.time()))
            })
    });
    Some(EventOccurrence {
        event: event.clone(),
        occurrence_date: date,
        starts_at,
        ends_at,
        title: event.title.clone(),
        time_zone: event.time_zone.clone(),
        all_day: event.all_day,
        location: event.location.clone(),
        notes: event.notes.clone(),
        is_override: false,
    })
}

fn resolve_local(tz: Tz, local: NaiveDateTime) -> Option<DateTime<Utc>> {
    match tz.from_local_datetime(&local) {
        LocalResult::Single(value) => Some(value.with_timezone(&Utc)),
        LocalResult::Ambiguous(first, second) => Some(first.min(second).with_timezone(&Utc)),
        LocalResult::None => {
            let before = (1..=180).find_map(|minutes| {
                let candidate = local.checked_sub_signed(Duration::minutes(minutes))?;
                match tz.from_local_datetime(&candidate) {
                    LocalResult::Single(value) => Some(value.offset().fix().local_minus_utc()),
                    LocalResult::Ambiguous(a, b) => Some(
                        a.offset()
                            .fix()
                            .local_minus_utc()
                            .max(b.offset().fix().local_minus_utc()),
                    ),
                    LocalResult::None => None,
                }
            })?;
            let after = (1..=180).find_map(|minutes| {
                let candidate = local.checked_add_signed(Duration::minutes(minutes))?;
                match tz.from_local_datetime(&candidate) {
                    LocalResult::Single(value) => Some(value.offset().fix().local_minus_utc()),
                    LocalResult::Ambiguous(a, b) => Some(
                        a.offset()
                            .fix()
                            .local_minus_utc()
                            .min(b.offset().fix().local_minus_utc()),
                    ),
                    LocalResult::None => None,
                }
            })?;
            let shifted = local.checked_add_signed(Duration::seconds(i64::from(after - before)))?;
            match tz.from_local_datetime(&shifted) {
                LocalResult::Single(value) => Some(value.with_timezone(&Utc)),
                LocalResult::Ambiguous(a, b) => Some(a.min(b).with_timezone(&Utc)),
                LocalResult::None => None,
            }
        }
    }
}
