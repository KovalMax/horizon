# Calendar Events and recurrence

Status: Implemented.

## Outcome

Authenticated Members can read persisted Calendar events across Spaces they can access. Owners and Editors can create, edit, and delete events in their Spaces. Recurring series produce stable calendar occurrences that can be edited or canceled individually or as a whole series. The calendar UI uses this API for signed-in accounts; signed-out sample data remains a separate prototype.

## Contract

- Events belong to one Space and have a title, start/end local times represented by instants, IANA timezone, all-day flag, optional notes/location, and optional daily/weekly/monthly recurrence rule.
- Reads are membership-scoped. Owners and Editors may write; Viewers may not. Non-members cannot read or change an event.
- Daily recurrence uses each calendar date from the start date. Weekly recurrence uses the start date's weekday. Monthly recurrence uses the same day-of-month and skips months that do not contain that day (for example, January 31 is followed by March 31).
- A series may repeat indefinitely or through an inclusive end date in the series timezone. The start date is the first occurrence. Count-based endings and selecting multiple weekdays are out of scope.
- Timed series preserve their scheduled local start and end times in the IANA series timezone. If a local time falls in a DST gap, move it forward by the gap; if it is ambiguous during a fall-back, use the earlier instant. Each boundary is resolved independently.
- All-day occurrences use the series timezone's calendar date; they do not shift dates for Members in other zones.
- Editing/canceling one occurrence stores an override/cancellation keyed by its scheduled local date. An override may change event fields, including its date/time, but it does not change later occurrences.
- Editing/canceling the whole series affects past and future generated occurrences. Existing overrides/cancellations are preserved for scheduled dates that remain in the updated series; exceptions for dates no longer generated are discarded. “This and following” is out of scope.
- API calendar-range reads expand recurrence in the requested date range and return occurrence identities separately from series identities. Pagination must not omit or duplicate occurrences.
- Reads and occurrence changes remain Space-scoped and role-authorized like ordinary events.
- To-dos and Saved places remain session-only.

## Test seams

- Authenticated HTTP against real PostgreSQL verifies persistence, scope, recurrence expansion, role authorization, occurrence overrides/cancellations, and series updates.
- Controlled-date/time-zone tests cover daily, weekly, monthly, end-date, DST gap/overlap, all-day, and pagination behavior.
- React calendar workflow verifies occurrence display and edit/cancel versus whole-series actions while keeping sample events out of signed-in Spaces.
