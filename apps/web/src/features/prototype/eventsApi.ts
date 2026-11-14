import { authenticatedFetch } from "../auth/api";
import type { EventItem } from "./domain";

type ApiCalendarEvent = {
  id: string;
  space_id: string;
  space_name: string;
  space_kind: "personal" | "shared";
  title: string;
  starts_at: string;
  ends_at: string | null;
  time_zone: string;
  all_day: boolean;
  location: string | null;
  notes: string | null;
  recurrence: "daily" | "weekly" | "monthly" | null;
  recurrence_ends_on: string | null;
  occurrence_date: string | null;
  occurrence_id: string | null;
  is_override: boolean;
};

export type CalendarEventInput = {
  title: string;
  startsAt: string;
  endsAt: string | null;
  timeZone: string;
  allDay: boolean;
  location: string | null;
  notes: string | null;
  recurrence: "daily" | "weekly" | "monthly" | null;
  recurrenceEndsOn: string | null;
};

export class CalendarEventsApiError extends Error {
  constructor(public readonly status: number) {
    super("Calendar events could not be loaded or saved.");
  }
}

type ApiEventPage = {
  items: ApiCalendarEvent[];
  next_offset: number | null;
};

function zonedParts(instant: Date, timeZone: string) {
  const parts = new Intl.DateTimeFormat("en-GB", {
    timeZone,
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hourCycle: "h23",
  }).formatToParts(instant);
  return Object.fromEntries(parts.map(({ type, value }) => [type, value]));
}

function dayOffset(date: string, today: string): number {
  const [year, month, day] = date.split("-").map(Number);
  const [todayYear, todayMonth, todayDay] = today.split("-").map(Number);
  return Math.round(
    (Date.UTC(year!, month! - 1, day!) -
      Date.UTC(todayYear!, todayMonth! - 1, todayDay!)) /
      86_400_000,
  );
}

function toEvent(event: ApiCalendarEvent): EventItem {
  const startsAt = new Date(event.starts_at);
  const startParts = zonedParts(startsAt, event.time_zone);
  const date = `${startParts.year}-${startParts.month}-${startParts.day}`;
  const todayParts = zonedParts(new Date(), event.time_zone);
  const today = `${todayParts.year}-${todayParts.month}-${todayParts.day}`;
  const endsAt = event.ends_at ? new Date(event.ends_at) : null;
  const endParts = endsAt ? zonedParts(endsAt, event.time_zone) : null;

  return {
    id: event.id,
    title: event.title,
    space: event.space_id,
    spaceName: event.space_name,
    spaceTone: event.space_kind === "personal" ? "personal" : "home",
    spaceIcon: event.space_kind === "personal" ? "person" : "home",
    day: dayOffset(date, today),
    time: event.all_day ? undefined : `${startParts.hour}:${startParts.minute}`,
    end: endParts ? `${endParts.hour}:${endParts.minute}` : undefined,
    allDay: event.all_day,
    place: event.location ?? undefined,
    notes: event.notes ?? undefined,
    repeat: event.recurrence
      ? `Every ${event.recurrence === "daily" ? "day" : event.recurrence === "weekly" ? "week" : "month"}`
      : undefined,
    occurrenceDate: event.occurrence_date ?? undefined,
    occurrenceId: event.occurrence_id ?? undefined,
    isOccurrenceOverride: event.is_override,
    recurrenceEndsOn: event.recurrence_ends_on ?? undefined,
    zone: event.time_zone,
    startDate: date,
    startsAt: event.starts_at,
    endsAt: event.ends_at ?? undefined,
    timeZone: event.time_zone,
  };
}

async function readEventsResponse(response: Response): Promise<EventItem> {
  if (!response.ok) throw new CalendarEventsApiError(response.status);
  return toEvent((await response.json()) as ApiCalendarEvent);
}

function serializeInput(input: CalendarEventInput) {
  return {
    title: input.title,
    starts_at: input.startsAt,
    ends_at: input.endsAt,
    time_zone: input.timeZone,
    all_day: input.allDay,
    location: input.location,
    notes: input.notes,
    recurrence: input.recurrence,
    recurrence_ends_on: input.recurrenceEndsOn,
  };
}

export async function listCalendarEvents(
  accessToken: string,
  offset = 0,
  startDate?: string,
  endDate?: string,
): Promise<{ items: EventItem[]; nextOffset: number | null }> {
  const range =
    startDate && endDate ? `&start_date=${startDate}&end_date=${endDate}` : "";
  const response = await authenticatedFetch(
    `/api/events?offset=${offset}${range}`,
    {
      headers: { Authorization: `Bearer ${accessToken}` },
    },
  );
  if (!response.ok) throw new CalendarEventsApiError(response.status);
  const page = (await response.json()) as ApiEventPage;
  return {
    items: page.items.map(toEvent),
    nextOffset: page.next_offset,
  };
}

export async function updateCalendarOccurrence(
  accessToken: string,
  eventId: string,
  occurrenceDate: string,
  input: CalendarEventInput,
): Promise<EventItem> {
  return readEventsResponse(
    await authenticatedFetch(
      `/api/events/${eventId}/occurrences/${occurrenceDate}`,
      {
        method: "PUT",
        headers: {
          Authorization: `Bearer ${accessToken}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify(serializeInput(input)),
      },
    ),
  );
}

export async function cancelCalendarOccurrence(
  accessToken: string,
  eventId: string,
  occurrenceDate: string,
): Promise<void> {
  const response = await authenticatedFetch(
    `/api/events/${eventId}/occurrences/${occurrenceDate}`,
    { method: "DELETE", headers: { Authorization: `Bearer ${accessToken}` } },
  );
  if (!response.ok) throw new CalendarEventsApiError(response.status);
}

export async function createCalendarEvent(
  accessToken: string,
  spaceId: string,
  input: CalendarEventInput,
): Promise<EventItem> {
  return readEventsResponse(
    await authenticatedFetch(`/api/spaces/${spaceId}/events`, {
      method: "POST",
      headers: {
        Authorization: `Bearer ${accessToken}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify(serializeInput(input)),
    }),
  );
}

export async function updateCalendarEvent(
  accessToken: string,
  eventId: string,
  input: CalendarEventInput,
): Promise<EventItem> {
  return readEventsResponse(
    await authenticatedFetch(`/api/events/${eventId}`, {
      method: "PUT",
      headers: {
        Authorization: `Bearer ${accessToken}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify(serializeInput(input)),
    }),
  );
}

export async function deleteCalendarEvent(
  accessToken: string,
  eventId: string,
): Promise<void> {
  const response = await authenticatedFetch(`/api/events/${eventId}`, {
    method: "DELETE",
    headers: { Authorization: `Bearer ${accessToken}` },
  });
  if (!response.ok) throw new CalendarEventsApiError(response.status);
}
