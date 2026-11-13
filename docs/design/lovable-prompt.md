# Horizon MVP — Lovable Prototype Prompt

Paste the prompt below into Lovable. It requests a **simulated prototype**, not a production app or a final technology decision. After generation, review the result on mobile and desktop against the design brief and design system.

---

Create a responsive, mobile-first **web prototype** for Horizon, a calm shared planner for households. It should also work for families and trip-planning groups without separate product modes. This is an invite-only beta concept, not an enterprise project-management tool.

## Prototype constraints

- Use clearly labeled illustrative local data and simulated interactions. Do not request real credentials, send real emails, invoke real payment or map services, or claim that authentication, import, notifications, recovery, or persistence are production-ready.
- Include realistic but fictional example Spaces: a personal Space, a household Space, and a trip Space. The same user can belong to multiple Spaces.
- Create coherent mobile and desktop layouts; prioritize the mobile browser experience without merely stretching it on desktop.
- Use accessible labels, keyboard navigation, visible focus, readable contrast, touch-friendly controls, and non-color indicators. Include responsive, loading, empty, failure, and offline examples where relevant.
- Treat all styling as an initial proposal for review, not a final brand or stack decision.

## Visual direction

Calm, welcoming, modern, and spacious. Use approachable sans-serif typography, a soft neutral canvas, white surfaces, a muted green primary accent, subtle borders, and restrained Space accents. Pair every Space color with a readable name. Avoid overly playful illustrations, dense enterprise tables, and decorative dashboard clutter.

Suggested initial colors to validate, not guaranteed accessible in every combination: canvas `#F7F8F5`, surface `#FFFFFF`, primary text `#202B29`, secondary text `#53635E`, primary accent `#2F6B5A`, and border `#D8E2DA`. Ensure all actual combinations meet applicable contrast requirements.

## Product rules to represent

- A Space contains its own calendar, To-do lists, and Saved places. Every item belongs to exactly one Space. A user can have a personal Space and join or create shared Spaces.
- A Space has one Owner. Viewers can read shared content; Editors can create, edit, and delete shared content; only the Owner manages membership and transfers ownership. Reflect the active user's role in the visible controls.
- All Spaces combines calendar events and Tasks, labeled and filterable by Space. Places are scoped to a selected Space, not an aggregate Places view.
- The first release is online-only. Show a clear offline state and do not pretend changes have been saved or queued while offline.

## Screens and flows

1. **Onboarding:** Illustrate email magic-link sign-in using simulated interactions. Show a personal Space created automatically and choices to create a shared Space, join one, or continue solo.
2. **Overview and navigation:** Provide an All Spaces calendar and task overview with Space labels and filters. Make switching to a selected Space obvious. Show useful first-use and empty states.
3. **Calendar:** Show timed and all-day events. Provide a creation flow for title, optional notes and location, timezone, and daily/weekly/monthly recurrence. Illustrate editing or canceling one occurrence versus the entire series. Show a member's local time and make the event timezone available.
4. **Tasks:** Show shared lists and open/completed Tasks. Provide creation with one optional assignee, optional due date/time, and multiple reminders. Illustrate the recipient's default time versus an explicitly configured time, and a personal reminder adjustment.
5. **Places:** In a selected Space, show categorized list and map views with filters and a directions handoff. Illustrate manual entry and a best-effort Google Maps-link import with editable review and a failure-to-manual-entry path. Use a clearly simulated map placeholder rather than implying a real maps integration. Keep the list usable when the map is unavailable.
6. **Members and sharing:** Show Owner, Viewer, and Editor roles. Illustrate email invitations and reusable join links with configurable expiry and revocation. Default an invite to Viewer. Show Owner approval of a link request with requester identity. Warn before sharing a personal Space that its existing content will become visible.
7. **Notifications and settings:** Illustrate in-app notifications, email/channel preferences, and per-Space mute. Show examples for assignment, due reminder, meaningful event change, membership change, and a join request/result. Do not represent push notifications as available at web launch.
8. **Safety:** Illustrate Owner transfer before leaving a Space that continues; allow Owner deletion after confirmation. Explain the seven-day restoration window and that pending invitations and links remain invalid after restoration.

Use a small set of well-connected representative flows rather than inventing product features. Do not add external calendar sync, child profiles, subtasks, task priority or recurrence, comments, detailed activity history, in-app place discovery, or offline synchronization.

Make the prototype easy to review: use plausible sample content, identify simulated states, and provide navigation to the main screens and key success/error states.
