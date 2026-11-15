# Horizon

Horizon helps people organize shared plans and reference information across personal and shared spaces.

## Language

**Space**:
A planning unit containing its own calendar, to-do lists, and saved places. A Space has one Owner, can have additional Members, and each item belongs to exactly one Space.
_Avoid_: Group, workspace

**Owner**:
The Member responsible for managing a Space's membership and transferring ownership.
_Avoid_: Admin

**Member**:
A user with an account who has access to a Space as its Owner, Viewer, or Editor.
_Avoid_: Collaborator, participant

**Viewer**:
A Member who can read all shared content in a Space but cannot change it.
_Avoid_: Read-only user

**Editor**:
A Member who can create, edit, and delete shared content in a Space but cannot manage membership.
_Avoid_: Writer

**Invitation**:
A request to join a Space, delivered by email or initiated through a share link. Joining requires an account; link-based requests require Owner approval.
_Avoid_: Invite link (when referring to the request rather than its delivery method)

**Calendar event**:
A scheduled item in a Space's calendar, anchored to a selected timezone and displayed in each Member's local time.
_Avoid_: Appointment

**Repeating event**:
A Calendar event that recurs daily, weekly, or monthly; an occurrence can be changed or canceled independently, or the entire series can be changed or canceled.
_Avoid_: Recurring appointment

**To-do list**:
A collection of tasks within a Space.
_Avoid_: Checklist

**Task**:
A piece of work in a To-do list that can have a due date and an assignee.
_Avoid_: To-do item

**Task reminder**:
A reminder scheduled relative to a Task's due date and delivered to its assignee, or to its creator when it has no assignee.
_Avoid_: Task alert

**Saved place**:
A location stored in a Space and organized by category.
_Avoid_: Bookmark

**Place category**:
A built-in grouping for Saved places within a Space. Custom categories are deferred.
_Avoid_: Tag (when referring to a Place's category)

**All Spaces view**:
A combined calendar and task overview across a Member's Spaces, with each item identified by its Space. Saved places remain scoped to their Space.
_Avoid_: Global view

**Notification**:
A message about a relevant planning or membership change, delivered in-app or by email according to a Member's preferences.
_Avoid_: Alert (unless referring to a system-level alert)
