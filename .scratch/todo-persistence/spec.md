# Shared To-do lists and Tasks

Status: Implemented for persisted Lists/Tasks, assignment, editing, and deletion. Task reminders and delivery remain deferred.

## Outcome

Authenticated Members can see persisted To-do lists and Tasks in the Spaces they can access. Owners and Editors can create, edit, move, assign, complete, and delete Tasks; they can also rename and delete Lists. Viewers can read but not write. Signed-out sample planner data remains session-only.

## Contract

- Persist Space-scoped list names and Tasks with title, optional due date/time, and open/completed state.
- A Task may be assigned to one current Member of its Space; signed-in assignment uses real Member identities and is validated on creation and update. Task edits validate title, due date/time, destination List, and optional assignee using create constraints. Reminder configuration and delivery remain out of scope; do not present sample people as real Space Members in the signed-in flow.
- List names are unique within a Space. Deleting a List explicitly confirms and permanently deletes its Tasks.
- Saved places remain session-only.
- Reads are membership-scoped; Owners and Editors may write; Viewers may not.

## Test seams

- Authenticated HTTP against real PostgreSQL verifies list/task persistence, updates, moves, assignment, completion, deletion/cascade, membership scope, conflicts, and role authorization.
- React To-do workflow verifies persisted loading, creation, editing, deletion confirmation, and completion feedback while keeping sample Tasks out of signed-in Spaces.
