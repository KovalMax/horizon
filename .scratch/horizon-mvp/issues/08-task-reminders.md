# 08 — Task reminders and email delivery

## Decisions

- Each Task defaults to reminders 1 day and 1 hour before its due date/time; an Editor can choose either or both.
- Timed Tasks use the recipient's local due timestamp minus the offset. Date-only Tasks use the recipient's default reminder time on the date that many whole calendar days before the due date (the 1-hour reminder is on the due date).
- Each account stores an IANA timezone and a default local reminder time (09:00 default). Validate timezone names against the IANA database.
- The recipient can privately override reminder offsets for a Task (none, 1 day, 1 hour, or both); this never changes the Task's shared default or another recipient's settings.
- Resolve the recipient as the assignee, or the creator when unassigned. On reassignment, the new recipient uses their own override if present, otherwise the Task defaults.
- Recompute pending reminders when due date/time, assignee, or shared offsets change, and when recipient timezone/default time changes. Completion and deletion cancel pending jobs. Reopening a Task schedules only future reminders.
- Due reminders honor the `due_reminder` in-app/email channel preferences (in-app on, email off by default) and Space mute. Muting suppresses delivery without later backfill.
- Use existing configured SMTP. Jobs are claimed atomically, retried with exponential backoff up to 8 attempts, and then retained as failed for operational visibility. Unique job keys prevent normal duplicate scheduling. SMTP is at-least-once at the crash boundary after remote acceptance and before local acknowledgment; exactly-once delivery cannot be guaranteed.
- Date-only local times falling in a DST gap are moved to the next valid local instant; ambiguous times use the earlier instant.

## Acceptance

- Scheduling is durable across API restarts and concurrent workers; changes and cancellation are transactional with Task updates.
- Channel opt-out, Space mute, private overrides, timezone/DST handling, transient retry, terminal failure, and duplicate-job prevention are covered by tests.
- The API exposes account reminder settings and private per-Task reminder preferences; Task forms expose shared offsets.
