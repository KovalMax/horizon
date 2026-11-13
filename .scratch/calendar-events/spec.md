# Calendar Events persistence

Status: Implemented.

## Outcome

Authenticated Members can read persisted Calendar events across Spaces they can access. Owners and Editors can create, edit, and delete events in their Spaces. The calendar UI uses this API for signed-in accounts; signed-out sample data remains a separate prototype.

## Contract

- Events belong to one Space and have a title, start/end instants, IANA timezone, all-day flag, optional notes/location, and optional daily/weekly/monthly recurrence rule.
- Reads are membership-scoped. Owners and Editors may write; Viewers may not. Non-members cannot read or change an event.
- Recurrence is stored as series metadata. Editing/canceling an individual occurrence and timezone conversion between Members' local zones are not part of this slice.
- To-dos and Saved places remain session-only.

## Test seams

- Authenticated HTTP against real PostgreSQL verifies persistence, scope, CRUD, and role authorization.
- React calendar workflow verifies API loading and create/edit/delete feedback while keeping sample events out of signed-in Spaces.
