# Horizon MVP: Product Requirements and Specification

Status: Draft — product scope agreed; technical feasibility and design remain to be validated.

## Problem Statement

Households, families, and groups planning trips need a convenient way to coordinate events, tasks, and useful places. Their plans are often spread across personal calendars, messages, lists, and map links, making it difficult to see what is shared, who is responsible, and what needs attention.

The initial audience is households. The same product should also serve families and trip-planning groups without requiring separate product modes.

## Solution

Horizon is a mobile-first planner built around Spaces: personal or shared planning units that each contain a calendar, to-do lists, and saved places. Users can invite others, control their access, and see calendar events and tasks across Spaces.

The MVP is a private beta of a responsive web application usable on mobile and desktop. Anyone can create an account with a verified email; joining another person's Space still requires a Space invitation or Owner approval. It requires an internet connection and provides clear offline and error states. Native mobile applications are a later phase.

## User Stories

### Access and onboarding

1. As a user, I want to sign in using an email magic link, so that I can access Horizon without a password.
2. As a user, I want to regain access through my verified email address, so that I can recover my account.
3. As a new user, I want to create an account with a verified email without a beta invitation, so that I can begin using Horizon.
4. As an invited person, I want to create or sign into an account before joining a Space, so that access is associated with my identity.
5. As a new user, I want a personal Space created for me, so that I can begin planning immediately.
6. As a new user, I want to create a shared Space, join a Space, or continue solo, so that onboarding suits my needs.

### Spaces and permissions

7. As a user, I want multiple Spaces, so that I can separate household, family, and trip plans.
8. As an Owner, I want a Space to contain its own calendar, lists, and places, so that its content stays organized.
9. As an Owner, I want to invite Members by email or share link, so that I can choose a convenient invitation method.
10. As an Owner, I want to select Viewer or Editor access for an invitation, defaulting to Viewer, so that access is intentional.
11. As an Owner, I want to change a Member's role, so that their permissions remain appropriate.
12. As an Owner, I want to manage membership, so that I control who can access my Space.
13. As a Viewer, I want to view shared content without modifying it, so that I can stay informed without accidental changes.
14. As an Editor, I want to create, edit, and delete shared content, so that I can participate in planning.
15. As a Member, I want only the Owner to manage membership and transfer ownership, so that administration is accountable.
16. As an Owner, I want reusable join links with configurable expiry and revocation, so that I can control access over time.
17. As an Owner, I want to review the requester's account identity and approve link-based join requests, so that a forwarded link does not silently grant access.
18. As a requester, I want to learn whether my join request was approved, so that I know whether I can access the Space.
19. As a personal Space Owner, I want a warning before sharing existing content, so that I understand what new Members will see.
20. As an Owner, I want to transfer ownership before leaving a Space that should continue, so that it always has one Owner.
21. As an Owner, I want to delete my Space after confirmation, so that I can remove it when it is no longer needed.
22. As a former Owner, I want to restore a deleted Space within seven days, so that I can recover from an accidental deletion.
23. As a former Owner, I want restored content and existing Member roles to return, so that the Space can continue where it left off.
24. As an Owner, I want pending invitations and join links invalidated on deletion, so that I can issue fresh access after a restoration.
25. As a Member, I want a combined calendar and task overview across my Spaces, with labels and filters, so that I can understand my commitments.

### Calendar

26. As an Editor, I want to create timed or all-day Calendar events with a title and optional notes and location, so that Members can coordinate plans.
27. As an Editor, I want to choose an event timezone, so that a plan is anchored to the appropriate location.
28. As a Member, I want to see event times in my local timezone and inspect the event timezone when relevant, so that I know when an event occurs.
29. As an Editor, I want events to repeat daily, weekly, or monthly, so that recurring household plans need not be recreated.
30. As an Editor, I want to edit or cancel a single occurrence or an entire series, so that I can adjust repeating plans.

### To-do lists

31. As an Editor, I want to create and manage shared To-do lists and Tasks, so that Members can coordinate work.
32. As an Editor, I want to assign a Task to one Member, so that responsibility is clear.
33. As an Editor, I want to set a due date and optional due time, so that deadlines can be as precise as needed.
34. As an Editor, I want to mark a Task open or completed, so that its progress is clear.
35. As an Editor, I want to configure one or more reminders relative to a Task's due date during creation, so that the responsible person is reminded.
36. As an assignee, I want Task reminders delivered to me, so that I can act on time.
37. As a Task creator, I want to receive reminders when a Task has no assignee, so that it is not forgotten.
38. As a reminder recipient, I want a default local time for date-only Tasks, so that reminders arrive at a useful time.
39. As an assignee, I want to adjust my own reminder without changing other people's preferences, so that reminders suit me.

### Saved places

40. As an Editor, I want to save a place manually with a name, address, optional notes, and contact details, so that useful locations are available to the Space.
41. As an Editor, I want to organize Saved places using built-in or Space-specific custom categories, so that they are easy to find.
42. As an Editor, I want to import a Google Maps link on a best-effort basis, so that I can avoid retyping available details.
43. As an Editor, I want to review and correct imported details before saving, so that incomplete or incorrect data does not become final without my consent.
44. As an Editor, I want a clear failure message and manual entry when import fails, so that I can still save the place.
45. As a Member, I want a categorized list and a map of the selected Space's Saved places, so that I can browse them in context.
46. As a Member, I want category filters and a list fallback if map data is unavailable, so that places remain accessible.
47. As a Member, I want to open a Saved place in a map application for directions, so that I can navigate there.

### Notifications and reliability

48. As a Member, I want in-app and email notifications for relevant assignments, due reminders, meaningful event changes, and membership changes, so that I stay informed.
49. As a Member, I want notification channel preferences and per-Space mute, so that I can control interruptions.
50. As a Member, I want affected people rather than the actor notified of shared changes, so that notifications are relevant.
51. As an Owner, I want join-request notifications, so that I can review pending requests.
52. As a user, I want clear offline and error states, so that I understand when actions cannot be completed.

## Implementation Decisions

### Product boundaries

- Every content item belongs to exactly one Space. Each Space has exactly one Owner and may have additional account-holding Members. Viewer access is read-only; Editors can create, edit, and delete shared content. The Owner manages membership and ownership.
- A personal Space may be shared after an explicit warning about visibility of existing content. Ownership transfers apply to shared Spaces only, go to an existing Member, and demote the outgoing Owner to Editor. An Owner may delete a personal or shared Space after confirmation; only the deleting Owner can restore it within seven days. Deletion immediately removes access, revokes pending invitations and links, and preserves pending join requests for review after restoration. Restoration returns content and existing Member roles without reactivating old invitations or links; an hourly background purge permanently removes Spaces after the recovery deadline.
- Space invitations support email and reusable links. The Owner selects Viewer or Editor (default Viewer) for a link; Owner access is never granted by a link. Link requests require sign-in and Owner approval; the Owner chooses an expiry timestamp or no expiry and can revoke the link. Revocation blocks new requests while already-pending requests remain reviewable. Only one request per account/Space may be pending; declined requesters may retry through a valid link. Requesters can see their decision status; durable notifications are handled separately.
- Events are anchored to a selected timezone and displayed in each Member's local timezone. Daily, weekly, and monthly recurrence supports editing or canceling one occurrence or the entire series. Detailed recurrence edge cases remain for feature-level specification.
- Tasks have open/completed status, at most one assignee, and an optional due date and time. Multiple reminders may be selected relative to a due date. The assignee receives reminders; the creator does when no assignee exists. An explicitly configured reminder time takes precedence; otherwise use the recipient's default and local timezone. Assignees can adjust their own reminder without changing others' preferences.
- Place import attempts to obtain name, address, coordinates, phone, and website when reliably available. It always presents an editable preview; ratings and photos are excluded. Import methods, supported links, provider terms, and costs require feasibility validation.
- Notifications use a persisted in-app inbox first; email delivery remains deferred. Channel preferences and Space mute control in-app delivery. Notify affected Members rather than the actor; Owners receive join requests and requesters receive approval results. Quiet hours are not required for launch.
- The beta is online-only with clear offline/error feedback. It does not promise offline edits or synchronization.

### Design before implementation

### Notification policy

- In-app events are Task assignments, membership role/removal changes, calendar event create/update/cancel changes, new join requests, and join-request outcomes. Due reminders are deferred.
- Never notify the actor. Assignment changes go to the new assignee; membership changes go to the affected Member; event changes go to all other active Space Members; new requests go to Owners; decisions go to the requester.
- Read state is independent per item and changes only through explicit mark-read/unread or mark-all-read actions. Opening the inbox does not mark items read.
- Retain notifications for 90 days. Muting suppresses new in-app delivery and unmuting does not backfill. In-app defaults on and email defaults off for each implemented category.

- Produce a design brief and design system covering visual foundations, reusable components, responsive layouts, and key flows before implementation.
- Direction: calm, welcoming, modern household planner; spacious layouts, approachable typography, and a restrained palette that distinguishes Spaces without an enterprise-project-management feel.
- Produce a Lovable prototype for core web flows after the brief and design system. No Lovable integration is currently available in this session; a ready-to-paste prompt is an alternative until access is provided. Do not assume generated prototype code is the production implementation.

### Technical choices and feasibility

- Use React with TypeScript for the web client. React Native is a possible future mobile approach, not a commitment; shared business logic does not guarantee shared UI. Rust/Actix and PostgreSQL are the preferred backend baseline. Redis is an option for demonstrated needs, not a launch dependency; browser/CDN caching may suffice for static assets, and shared-data caches require explicit invalidation and authorization boundaries.
- Use Docker and Docker Compose for local development. Deploy the beta to a virtual machine via SSH and run a production Compose file. The VM provider, exact deployment procedure, secrets management, backups, TLS, monitoring, and recovery procedures remain to be designed and validated; Compose alone does not provide them.
- Proposed capability boundaries include identity and beta access; Spaces and membership; calendar and recurrence; lists, Tasks, and reminders; Saved places and import; notifications; and the responsive client/design system. These are product capabilities, not a predetermined deployment architecture.
- Define provider choices and operational requirements for magic-link email, notification delivery, map display, and place import during technical discovery. Validate provider terms, data handling, cost, and failure behavior.
- No implementation code or test suite exists in this project yet; APIs, schema, infrastructure, and concrete module interfaces remain to be designed.

## Testing Decisions

- Test externally observable behavior rather than implementation details. Prefer the highest practical application interface as the primary seam, covering end-to-end business flows such as Space creation, permissions, invitations, recurring events, reminders, place import, and deletion/recovery.
- Substitute external email, map, and import services at their boundaries to keep tests reliable; verify integration behavior separately where feasible. Include timezone and clock-controlled cases for recurrence, reminders, invitation expiry, and the seven-day recovery window.
- Test authorization for each role, including attempts to access or modify content outside a Member's Space. Exercise deletion and restoration, invitation revocation, notification preferences, import failure, and offline/error states.

## Comments

- Account creation during the private beta is not gated by a beta or Space invitation. A verified email is required; invitations and Owner-approved requests govern membership in other people's Spaces.
- Add browser-level checks for key user journeys and responsive mobile and desktop layouts. Define accessibility checks as part of design and implementation planning.
- There is no existing test prior art in this project. Choose tools and exact interfaces during technical design; use test-driven development in incremental vertical slices during implementation.

## Out of Scope

- Native iOS and Android apps and native push notifications in the initial beta.
- Offline editing or synchronization, external calendar sync, complex recurrence rules, and editing this-and-following event occurrences.
- Task subtasks, priorities, and task recurrence; comments and detailed activity history.
- Child profiles without accounts, in-app place discovery, imported ratings or photos, and quiet-hours requirements.
- Password and social sign-in for the beta, and guarantees that every Google Maps link can be imported.
- Final stack selection, production implementation, fixed launch date, and numeric beta success targets at this stage.

## Further Notes

- Success is evaluated through activation and sustained shared use: a user creates or joins a Space, adds shared content, and at least one other Member returns and contributes within the first few weeks. Define numeric targets once the beta cohort is known.
- Research and technical discovery should resolve provider feasibility, privacy and retention requirements, detailed event recurrence behavior, reminder scheduling edge cases, and implementation architecture before feature-level work is considered ready.
- This is an agreed product-direction draft, not an implementation-ready ticket. It is intentionally not marked `ready-for-agent` until the remaining feasibility and design work is complete.
