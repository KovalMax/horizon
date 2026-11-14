# Slice 7: Durable notification foundation

Status: Complete.

## Decisions

- Persist in-app notifications for task assignments, membership changes, calendar event changes, new join requests, and join-request outcomes. Due reminders are deferred.
- Never notify the actor. Assignment changes notify only the new assignee; role/removal changes notify the affected Member; event changes notify every other active Space Member; join requests notify the Space Owners; decisions notify the requester.
- Each item has independent read state. Users explicitly mark one item read/unread or mark all read; opening the inbox does not mark items read.
- Keep notifications for 90 days and purge expired records automatically.
- Muting a Space suppresses new in-app notifications. Unmuting does not backfill suppressed events.
- In-app preferences default on for all implemented categories; email preferences default off. Email preference values persist, but email delivery and due reminders are not implemented in this slice.
- Inbox reads are authenticated and paginated, and notification state/preferences are scoped to the signed-in user.

## Acceptance

- Notifications are written in the same database transaction as the event that produces them.
- Cover assignment, membership role/removal, event changes, new join requests, and request decisions, including actor suppression and recipient rules.
- Test paginated inbox reads, individual/all read-state updates, preference defaults and updates, per-Space mute, and 90-day expiration.
- Replace the placeholder inbox with an accessible loading/empty/error-ready interface and persist channel/mute settings.
