# 02 — Edit and remove Tasks and Lists

Status: Complete.
Blocked by: None.

## Outcome

Owners and Editors can update or remove Tasks and To-do Lists in their Space. Viewers keep read-only access. Task edits include reassignment and moving a Task to another List. Deleting a List requires confirmation and deletes its contained Tasks.

## Contract

- Task edits validate title, dates/times, destination List, and optional same-Space Member assignee using the existing create constraints.
- Task deletion removes only that Task.
- List rename is unique within its Space; a duplicate name is rejected without changing data.
- List deletion is an explicit destructive action: after confirmation, the List and all its Tasks are deleted transactionally.
- Every read/write remains membership-scoped; only Owners and Editors can mutate. Unknown/inaccessible resources do not disclose another Space's data.

## Verification

- PostgreSQL-backed API tests cover update, move, reassignment/unassignment, delete, duplicate rename, cascade deletion, and Viewer denial.
- React workflow tests cover editing and deleting a Task and safely confirming deletion of a non-empty List.
- `sh scripts/check.sh` passes.

## Completion

- Added authenticated Task update/delete and List rename/delete endpoints with Owner/Editor enforcement and OpenAPI registration.
- Task update supports moving Lists and changing/unsetting same-Space assignees. Duplicate List names return `409 Conflict`.
- Non-empty List deletion cascades to Tasks; the UI names this permanent cascade in its confirmation prompt.
- Verified through PostgreSQL and React workflow tests; `sh scripts/check.sh` passes.
