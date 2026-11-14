# Horizon MVP delivery roadmap

Status: Proposed — sequence and scope for review.

This roadmap reconciles the current implementation with `.scratch/horizon-mvp/spec.md` and the feature specs. It proposes implementation slices; it does not convert unresolved product or provider decisions into assumed requirements.

## Assessment: where the MVP stands

| Area | Assessment | Evidence / remaining gap |
| --- | --- | --- |
| Foundation | **Implemented locally; external verification pending.** | React/TypeScript, Rust/Actix, PostgreSQL, Compose, migrations, readiness, CI config, and containerized checks exist. GitHub-hosted CI has not been verified against the user's repository. Foundation issue statuses are stale. Production hosting/operations are not done. |
| Authentication | **Core flow implemented; launch details pending.** | Magic-link request/redeem, account lookup, access/refresh lifecycle, sign-out, Mailpit, and frontend workflow are present. Browser/Mailpit end-to-end verification and a production mail provider are pending. Session listing/revoke-all is explicitly deferred. |
| Spaces and membership | **Membership and ownership/deletion lifecycle implemented.** | Personal/shared Space creation, Member directory, invitations, role changes/removal/revocation, join links/requests, shared-Space ownership transfer, and seven-day soft-delete/restore/purge are covered locally. Personal-Space sharing warning needs browser verification. |
| Calendar | **Basic persistence and CRUD implemented.** | Space-scoped event create/read/update/delete, timezone metadata, and recurrence metadata exist. Recurrence is not a series/occurrence experience; local-time display and DST behavior need product and browser verification. |
| To-dos | **Persistence, assignment, management, and due reminders implemented locally.** | Owners/Editors can create, update, move, assign, complete, and delete Tasks; rename/delete Lists. Viewers remain read-only. Reminder delivery uses the durable worker and configured SMTP; production email verification remains pending. |
| Saved Places | **Manual persistence implemented.** | Name, built-in category, address, and note can be saved and read. Map-provider integration/directions, contact fields, custom categories, and Maps-link import are deferred. Verify that signed-in data has a real map experience rather than prototype-only map presentation. |
| Notifications | **Durable in-app foundation and due-reminder delivery implemented locally.** | Persisted inbox, read-state APIs, assignment/membership/event/join-request notifications, per-event channel preferences, per-Space mute, due-reminder email, and 90-day cleanup are implemented. Production email verification remains pending. |
| Bounded reads | **Implemented and validated locally.** | Events, Tasks, Lists, and Places have bounded offset pages and visible load-more controls. Page behavior is covered by PostgreSQL tests. |
| SQL review | **Applied and covered by the current check.** | Invitation lifecycle/re-invitation and index/query alignment fixes are in the uncommitted work. |
| Design, accessibility, and browser QA | **Not yet demonstrated as complete.** | The MVP spec calls for a design brief/system, responsive layouts, accessibility checks, and browser-level journeys. Existing React workflow tests are useful but do not replace those checks. |
| Beta operations | **Not started.** | Production mail, VM/Compose deployment, TLS/secrets, backups and restore drills, monitoring, production migrations, privacy/retention, and account deletion policy remain unresolved. |

The current local verification baseline is `sh scripts/check.sh` (Rust format/Clippy/tests, web format/build/tests, PostgreSQL-backed integration tests); it passed after the pagination changes. It does not verify a production deployment or a real browser/Mailpit journey.

## Recommended sequence

Slices are deliberately vertical: each should include the data/API boundary, visible user flow, authorization, and tests needed for that behavior. Keep each slice independently reviewable and update the corresponding feature spec when its contract changes.

### 0. Reconcile status, decisions, and design

**Outcome:** There is one trustworthy view of what is implemented and what the beta promises.

- Update stale foundation/auth issue statuses and clarify that Space-level authorization is implemented across current endpoints, while unimplemented membership operations remain explicit.
- Choose the MVP design brief/system direction and review the current prototype against mobile/desktop, loading/error/empty, and permission states.
- Confirm recurrence semantics, Task reminder requirements, place-provider constraints, and beta privacy/retention decisions before their dependent implementation slices.
- Add browser-level test tooling/approach to the plan; do not treat the current unit/workflow suite as browser verification.

**Exit:** Decisions that affect schemas/providers are recorded in the relevant specs; implementation and deferred work are accurately labeled. This can run alongside backend slices that do not depend on unresolved decisions.

### 1. Persist real Task assignment — complete

**Outcome:** An Editor can assign a Task to one real Member of that Task's Space; the assignee is visible after reload.

- Persist an optional assignee identity and expose it through the Task API.
- Populate the selector from the real Space Member directory, support unassigning, and show a clear unassigned state.
- Validate membership in the same Space on create/update; reject non-members and cross-Space IDs. Enforce Editor/Owner write permissions and read-only Viewer behavior.
- Cover persistence, authorization, invalid assignment, and the signed-in UI flow with PostgreSQL/API and React workflow tests.

**Depends on:** Existing real Member directory and Task persistence (both present). Keep delivery/reminders out of this slice.

**Implemented:** Optional assignee persistence, same-Space membership validation in the Task transaction, real Member selection in the signed-in form, and assignee display. PostgreSQL/API tests cover same-Space assignment, unassigned Tasks, cross-Space rejection without an orphan List, and Viewer write denial; the React workflow verifies selecting and seeing a real assignee.

### 2. Complete basic Task and List management

**Status: Complete.**

**Outcome:** Editors can correct or remove their shared planning data instead of being limited to creation and completion toggles.

- Task title, due-date/time, List, and assignee edits; Task deletion; and List rename/deletion are implemented.
- Non-empty List deletion permanently deletes its contained Tasks after explicit confirmation.
- Owner/Editor enforcement, Viewer denial, duplicate-name conflicts, cascading deletion, and CRUD workflows are covered by PostgreSQL and React tests.

**Depends on:** Slice 1 for assignment editing; otherwise can be split if assignment is deferred.

### 3. Define and implement Calendar recurrence

**Status: Complete.**

**Outcome:** Recurring events behave predictably across time zones and daylight-saving transitions, not merely carry a recurrence label.

- First publish a feature-level recurrence contract: which weekdays/month-days are supported, end conditions, all-day behavior, timezone anchoring, DST gaps/overlaps, and edits/cancellations for one occurrence versus the series.
- Implement occurrence generation and persistence semantics, then expose a useful calendar range without duplicating or losing events across pagination.
- Add tests with controlled dates/time zones for daily, weekly, and monthly rules, DST, edits, cancellation, and pagination; verify the visible calendar in a browser.
- Keep “this and following” out of scope, consistent with the MVP spec, unless the product explicitly changes that boundary.

**Depends on:** Product decisions in slice 0. This is a larger domain slice; split the recurrence contract from implementation if review reveals significant edge cases.

### 4. Membership administration basics — complete

**Outcome:** Owners can keep current Space access accurate.

- Owners can change non-Owner Members between Viewer and Editor, remove non-Owner Members, and revoke pending invitations. Owner changes/removals are blocked until ownership transfer is implemented.
- Revoked/expired invitations are terminal and their addresses can be safely re-invited; pending duplicates conflict.
- Personal Space Owners receive a warning before the first invitation from that browser, before new Members can see existing content.
- Owner-only operations and PostgreSQL authorization/lifecycle behavior are covered by API tests; the signed-in UI reflects successful changes immediately.

**Depends on:** Existing membership and invitation model. Ownership transfer is intentionally separate because it has different lifecycle/confirmation risks.

### 5. Join links and approval requests

**Status: Complete.**

**Outcome:** A signed-in non-member can request access through a controlled link, and an Owner can approve or decline the identified requester.

- Decide link expiry/reuse/revocation behavior, request uniqueness, requester visibility, and notification behavior.
- Add durable link/request state and Owner review/decision UI; acceptance must add the account as the specified role only after approval.
- Test expired/revoked links, repeated requests, decisions, race conditions, and non-member isolation.

**Implemented:** Reusable Viewer/Editor links with optional expiry and revocation; authenticated requests; one pending request per account/Space; Owner approval/decline; requester-visible status history; notification delivery remains in slice 7.

**Depends on:** Slice 4 membership administration and slice 7 notification foundation for user-facing request/decision updates (or ship a basic status page first and add notifications later).

### 6. Space ownership and deletion lifecycle

**Status: Complete.**

**Outcome:** Owners can transfer or end a Space without orphaning access or accidentally destroying recoverable data.

- Specify ownership transfer, confirmation, deletion grace period, restoration authorization, and permanent deletion behavior.
- Implement transfer first; implement soft-delete/restore and scheduled permanent deletion as a separate migration/API/UI change.
- Invalidate pending invitations/links on deletion; restore content and existing roles but do not reactivate old access grants, per MVP policy.
- Test sole-owner invariants, seven-day boundary with a controlled clock, restoration, expiry cleanup, and authorization.

**Implemented:** Shared-Space transfer to an existing Member (outgoing Owner becomes Editor); personal-Space transfer is blocked. Owners can soft-delete either Space type; only the deleting Owner can restore within seven days. Memberships are snapshotted and removed during deletion, invitations/links are revoked, pending requests remain pending, and an hourly API task purges expired Spaces.

**Depends on:** Slice 4 and the lifecycle policy in slice 0. Treat destructive and scheduled cleanup behavior as a separately reviewed slice.

### 7. Durable notification foundation and preferences

**Status: Complete.**

**Outcome:** Notification events and user choices have real persistence and a visible in-app inbox before email reminders are enabled.

- Decide event types, actor suppression, affected recipients, read/unread behavior, retention, per-Space mute, and per-channel preferences.
- Persist notifications and preferences; provide authenticated, paginated inbox/read-state APIs and accessible UI.
- Add notifications for Task assignment and membership changes first; then add event changes and join-request outcomes as their workflows land.
- Test recipient selection, actor exclusion, mute/preferences, duplicate prevention, read state, pagination, and cross-Space isolation.

**Implemented:** Notifications are written transactionally with Task assignment changes, membership role/removal changes, calendar event changes, join requests, and join-request decisions. The actor is suppressed; recipient rules, independent read state, 90-day retention, mute-without-backfill, and default channel choices are documented in `.scratch/horizon-mvp/issues/07-notifications.md`. Authenticated paginated inbox/read APIs and settings APIs are live; email delivery and due reminders remain out of scope.

**Depends on:** Slice 1 for assignments; slice 4/5 for membership event types. Keep email transport out until reliable in-app behavior and provider choice are established.

### 8. Task reminders and email delivery

**Status: Complete locally; production email verification remains pending.**

**Outcome:** Assignees (or the creator for an unassigned Task) receive configured reminders at the intended local time.

- Specify reminder offsets, date-only default local time, per-recipient timezone, edits/cancellation, completed/deleted Task behavior, delivery retry, and duplicate prevention.
- Build the scheduling/delivery boundary and operational visibility; choose the production email provider and define failure/retry policy before enabling external sends.
- Let assignees adjust their own reminder without mutating another Member's preference. Respect notification channels and Space mute as specified.
- Verify scheduling with a controlled clock and fake sender; test retries, DST, stale jobs, duplicate delivery, and opt-out behavior.

**Implemented:** Shared 1-day/1-hour offsets, per-recipient private overrides, local timezone/default reminder time, DST gap/overlap resolution, durable job reconciliation and atomic claiming, SMTP/in-app delivery, exponential retry with an eight-attempt limit, stale-claim recovery, mute/channel suppression, and lifecycle cancellation/reactivation. PostgreSQL integration coverage verifies transient retry/idempotent delivery, completion/reopen, no-backfill, mute suppression, and private override authorization/isolation; timezone gap/overlap behavior has deterministic unit coverage. `sh scripts/check.sh` passes locally. Real SMTP/browser journeys and production email operations are not verified.

**Depends on:** Slices 1, 2, and 7; production email decision from slice 0. Do not add a scheduler/cache service without a documented operational need and failure model.

### 9. Saved Places: map, directions, and richer manual data

**Outcome:** Members can browse real Saved Places in list/map form and navigate to a place, with a usable list fallback.

- Compare map providers and confirm terms, privacy, cost, browser-key restrictions, and failure behavior before choosing one.
- Persist coordinates and optional phone/website where reliably available; make map pins and directions use validated place data.
- Retain category filters and an accessible list fallback when map data/provider is unavailable.
- Test no-coordinate and provider-failure behavior, responsive map/list switching, and external directions links.

**Depends on:** Provider decision from slice 0. Custom categories can follow as a separate slice if user evidence justifies them.

### 10. Maps-link import

**Outcome:** An Editor can paste a supported Maps link, review an editable best-effort preview, and save or fall back to manual entry.

- Identify supported link formats and a compliant import/geocoding source; reject unsupported links safely.
- Keep imported data provisional until user confirmation; do not silently overwrite manual place details.
- Show useful partial-success/failure states and preserve manual entry as the fallback.
- Test supported/unsupported links, provider failure, malformed results, and correction-before-save.

**Depends on:** Slice 9 provider/terms decision and richer place fields. Ratings/photos remain excluded per MVP spec.

### 11. Responsive, accessible end-to-end acceptance

**Outcome:** The critical signed-in journeys are verified in a real browser at mobile and desktop layouts.

- Cover account request/redeem/sign-out through Mailpit, onboarding/Space creation, invitations, role permissions, shared Tasks/assignment, Calendar recurrence, Places, and failure/retry states.
- Check keyboard access, labels/focus, contrast, dialogs, loading/empty/error states, and responsive navigation against the approved design system.
- Keep external services substituted in deterministic tests; have a separate small smoke check for the real local browser-to-API-to-PostgreSQL path.

**Depends on:** The user journeys selected for beta and the relevant feature slices. Begin test-tool setup in slice 0 rather than waiting until the end.

### 12. Private-beta operational readiness

**Outcome:** A controlled beta can be deployed, recovered, and supported without relying on local-development assumptions.

- Select VM/provider, production email sender/provider, DNS/TLS, secret storage/rotation, and production Compose configuration.
- Define least-privilege networking, health checks, logs/metrics/alerts, abuse-limit review, and incident response.
- Automate backups and demonstrate restore into a clean environment; document migration/rollback constraints and retention/deletion behavior.
- Review auth/session security, privacy/data retention, account deletion/email change, dependency/security updates, and access to operational data.
- Run a staged deployment and rollback/restore exercise before inviting real household data.

**Depends on:** Provider/retention decisions, browser acceptance, and a confirmed beta cohort. This is a launch gate, not an optional polish task.

## Suggested near-term order

1. Reconcile spec/status drift and review design/decisions (slice 0; keep it time-boxed).
2. Specify Calendar recurrence while implementing the independent membership administration baseline (slices 3 and 4 can proceed in parallel after the recurrence contract is approved).
3. Build durable in-app notifications and preferences (slice 7), now that assignments, membership changes, and join-request outcomes are available; keep scheduled reminders/email (slice 8) behind that foundation.
4. Complete Space lifecycle (slice 6) and Places provider/import slices (9–10) based on beta priority and provider decisions.
5. Run browser acceptance (slice 11) continuously and close operational gates (slice 12) before the private beta.

This ordering prioritizes shared coordination value and security boundaries first. It is not a promise that every MVP story must ship in this exact sequence; provider-dependent work should not start before its feasibility gate is resolved.

## Planning notes

- Update `.scratch/horizon-mvp/spec.md` from its current “draft/not implementation-ready” wording only after scope/design/feasibility decisions are actually resolved.
- Feature specs are useful records, but not all are current. In particular, the foundation issue statuses and parts of the authentication delivery sequence predate the implemented Spaces/content authorization work.
- Create numbered issue files under `.scratch/<feature>/issues/` from the approved next slice rather than turning this roadmap into a large implementation ticket.
- Revisit this roadmap after the first beta cohort; usage should determine whether Maps import, custom categories, and notification channels beyond the initial need are worth the added complexity.
