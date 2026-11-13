# 01 — Persist real Task assignment

Status: Complete.
Blocked by: none; requires the existing Space Member directory and persisted Tasks.

## Outcome

An Owner or Editor can assign a Task to one current Member of the same Space, or leave it unassigned. The assignment survives reloads and is visible in the signed-in Task views. Viewers can read assignments but cannot create or change Tasks.

## Contract

- A Task has zero or one assignee, represented by the assignee's internal User ID.
- Assignees are selected from the authenticated Space's real Member directory; fictional prototype people are never options in a signed-in Space.
- Any current Member role (Owner, Editor, or Viewer) may be assigned.
- Assignment is checked against the Task's Space when writing. Unknown Users and Users who belong only to another Space are rejected without creating a Task or orphaned To-do list.
- Clearing the selection creates an unassigned Task. Assignment/reminder delivery and assignee-specific notification preferences are out of scope.

## Verification

- PostgreSQL-backed API tests cover same-Space assignment, unassigned Tasks, rejecting a cross-Space/non-member assignee, Viewer read access, and Viewer write denial.
- React workflow tests select a real Member, submit the assignment, and display the persisted assignee in the Task row.
- OpenAPI, formatting, Clippy, Rust tests, web build, and web tests pass through `sh scripts/check.sh`.
