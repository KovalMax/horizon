# Horizon MVP Design Brief

Status: Initial brief for exploration; visual choices and interaction details require review.

## Prototype review outcome

The current prototype is approved as a direction, not as a complete or production-ready design. The All Spaces view shows events and Tasks across Spaces; the dedicated Space stacks its Calendar, Tasks, and Places cards vertically on desktop as well as mobile. Each card has its own add action and can expand independently, with at most one expanded at a time. Membership is secondary, available through a small People control and modal. Keep navigation free of duplicate Space entries and place the return to All Spaces above the Space heading.

The current month calendar, map, other sample Spaces, authentication, import, and persistence are illustrative or incomplete. Validate accessibility and real user flows before treating prototype details as final.

## Product and audience

Horizon is a planner for households to coordinate events, tasks, and useful places in personal and shared Spaces. The same experience should also work for families and groups planning trips, without separate product modes.

The first release is an invite-only, online-first responsive web beta. Design for mobile browsers first, then adapt the experience for desktop. Native mobile apps are planned later, but are not part of this design delivery.

## Design goals

1. Make it easy to understand what needs attention across multiple Spaces.
2. Make shared planning feel welcoming and manageable, not like enterprise project management.
3. Make ownership, access, and the consequences of sharing clear before users act.
4. Keep core tasks quick on a phone without reducing desktop to an enlarged mobile screen.
5. Provide understandable feedback for loading, empty, offline, permission, and failure states.

## Visual direction

The agreed direction is **calm, welcoming, and modern**: spacious layouts, approachable typography, and a restrained palette that distinguishes Spaces without visual clutter. Space identity should remain understandable without relying on color alone.

Exact colors, typefaces, iconography, imagery, and brand assets are not yet decided. Explore them in the design system rather than treating a prototype's initial choices as final.

## Key concepts

- **Space:** A personal or shared planning unit with its own calendar, To-do lists, and Saved places. Each item belongs to exactly one Space.
- **Owner:** Manages Space membership and ownership; there is one Owner per Space.
- **Viewer and Editor:** Members who can respectively read shared content or create, edit, and delete it. Only the Owner manages membership.
- **All Spaces view:** A combined overview of events and Tasks with Space labels and filters. Saved places stay scoped to a selected Space.

See the project glossary and MVP specification for the full product language and agreed scope.

## Core journeys to design

### 1. Sign-in and onboarding

- Sign in or create an account through an email magic link.
- Show that a personal Space is created automatically.
- Offer clear paths to create a shared Space, join one, or continue solo.
- Show meaningful loading, link-expired, and recovery states; final authentication details remain subject to technical discovery.

### 2. Space selection and overview

- Navigate between the All Spaces overview and a selected Space.
- Show Space identity with a readable label as well as any visual accent.
- Make calendar and Task filters understandable on both mobile and desktop.
- Present useful first-use and empty states without inventing product functionality.

### 3. Calendar

- Browse Space events and the combined calendar.
- Create timed and all-day events with a title, optional notes and location, and a selected timezone.
- Show daily, weekly, and monthly recurrence and the choice to edit or cancel one occurrence or the entire series.
- Make local display time clear and expose the event timezone when relevant. Detailed recurrence edge cases remain open.

### 4. To-do lists and reminders

- Browse lists and create or complete Tasks.
- Support one optional assignee, an optional due date and time, and one or more reminders.
- Distinguish an explicit reminder time from a recipient's default. Allow an assignee to adjust their own reminder without implying that everyone's settings change.

### 5. Saved places

- Browse places in the selected Space as a categorized list or map, with category filters.
- Create a place manually with name, address, and optional notes and contact details.
- Enter a Google Maps link and review or correct any retrieved details before saving; show a clear failure and manual fallback.
- Offer a handoff to a map application for directions, and retain a usable list when map data is unavailable.

### 6. Membership and safety

- Invite by email or reusable link; choose Viewer or Editor, defaulting to Viewer.
- Show link expiry and revocation controls, and give the Owner a clear approval flow with the requester's identity.
- Warn before a personal Space is shared that existing content will become visible.
- Present transfer of ownership, confirmed Space deletion, and the seven-day recovery window clearly. Restored content and Member roles return, but invalidated invitations and links do not.

### 7. Notifications and preferences

- Show in-app notifications and email preferences, plus per-Space mute.
- Design for assignments, due reminders, meaningful event changes, membership changes, and join requests/results; do not imply every routine edit generates a notification.
- Push and quiet hours are not launch requirements.

## Responsive and accessibility principles

- Start with small-screen task flows; progressively use desktop space for orientation and comparison rather than merely scaling up the mobile layout.
- Keep navigation, controls, text, and feedback usable at varying viewport sizes and text scales. Exact breakpoints and component behavior remain design decisions.
- Support keyboard operation, visible focus, readable contrast, meaningful labels, and non-color indicators. Validate with accessibility checks and assistive technology during implementation.
- Confirm destructive actions and explain their consequences. Avoid exposing content to newly invited Members without the agreed warning.
- Because the MVP is online-only, show a clear offline state and do not imply that edits have been saved or queued when they have not.

## Design system deliverables

Before implementation, develop and review:

- Visual foundations: color, typography, spacing, elevation, iconography, and Space identity.
- Reusable components and states: navigation, buttons, inputs, date/time controls, lists, cards, filters, dialogs, notifications, empty states, and error states.
- Responsive layout patterns for mobile and desktop.
- Accessibility and interaction guidance, including focus, validation, destructive actions, and permission-dependent controls.
- Representative screens for the core journeys above, with enough detail to review behavior as well as appearance.

## Prototype and review

Create a Lovable prototype after the brief and initial design system are reviewed. It should demonstrate the core web flows with illustrative data and make clear which interactions are simulated. Prototype code and provider integrations are not assumed to be production-ready or to determine the final technology stack.

No Lovable integration is currently available in this session. A ready-to-paste prompt can be prepared for use in Lovable, and screenshots or a shareable result can then be reviewed together.

Review the prototype on mobile and desktop for clarity of Space context, task completion, permissions, readability, and error recovery. Changes to product behavior should be reconciled with the MVP specification rather than silently introduced by the prototype.

## Open design decisions

- Specific palette, typography, iconography, and brand identity.
- Navigation structure and responsive breakpoints.
- Calendar density and event-creation interaction patterns.
- Layout and placement of member, notification, and reminder controls.
- Exact content and interaction for map/import failure, authentication recovery, and deletion recovery.
- Accessibility validation targets and testing plan.

Technical feasibility, provider terms and costs, and the final client stack are separate decisions and may constrain the design after discovery.
