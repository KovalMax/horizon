export type Space = {
  id: string;
  name: string;
  role: "Owner" | "Editor" | "Viewer";
  members: number;
  tone: string;
  icon: string;
};

export type EventItem = {
  myReminderOffsetsMinutes?: number[];
  id: string;
  title: string;
  space: string;
  day: number;
  time?: string;
  end?: string;
  allDay?: boolean;
  place?: string;
  repeat?: string;
  zone?: string;
  notes?: string;
  spaceName?: string;
  spaceTone?: string;
  spaceIcon?: string;
  startDate?: string;
  startsAt?: string;
  endsAt?: string;
  timeZone?: string;
  occurrenceDate?: string;
  occurrenceId?: string;
  isOccurrenceOverride?: boolean;
  recurrenceEndsOn?: string;
};

export type TaskItem = {
  id: string;
  title: string;
  space: string;
  day?: number;
  time?: string;
  assigneeId?: string;
  assigneeEmail?: string;
  reminders?: number;
  reminderOffsetsMinutes?: number[];
  myReminderOffsetsMinutes?: number[];
  done?: boolean;
  list: string;
  listId?: string;
  spaceName?: string;
  spaceTone?: string;
  spaceIcon?: string;
};

export type PlaceItem = {
  id?: string;
  name: string;
  category: string;
  address: string;
  note?: string;
  space: string;
  spaceName?: string;
  spaceTone?: string;
};

export const dateAt = (days: number) => {
  const date = new Date();
  date.setHours(12, 0, 0, 0);
  date.setDate(date.getDate() + days);
  return date;
};

export const dateLabel = (days: number, long = false) =>
  dateAt(days).toLocaleDateString(
    "en-GB",
    long
      ? { weekday: "long", day: "numeric", month: "long" }
      : { weekday: "short", day: "numeric", month: "short" },
  );

export const todayLong = dateAt(0).toLocaleDateString("en-GB", {
  weekday: "long",
  day: "numeric",
  month: "long",
});
