# Horizon MVP Design System

Status: **Proposed starting point**, not an approved brand or implementation specification. Validate the tokens and patterns in mobile and desktop prototypes, including accessibility checks, before committing to a client stack or production implementation.

## Direction approved in prototype review

- Keep the combined All Spaces event and Task overview distinct from each dedicated Space, without duplicate Space navigation entries.
- Stack Calendar, Tasks, and Places cards vertically at both mobile and desktop sizes. Give each its own add action and allow one card to expand at a time.
- Show membership through a compact People control and modal, secondary to planning. Place the return to All Spaces above the Space heading.
- Use the prototype's calm Material-inspired surfaces and restrained tactile controls as a visual direction, not as verified Material compliance or finalized tokens. Browser and accessibility validation remain outstanding.

## Principles

1. **Calm and useful:** emphasize the next meaningful action, not dashboards full of decoration.
2. **Shared with intention:** make the current Space, permissions, and sharing consequences visible.
3. **Mobile first, desktop considered:** keep frequent actions reachable on small screens and use larger screens for context and comparison.
4. **Accessible by default:** use readable text, keyboard-accessible controls, visible focus, and labels or icons alongside color.
5. **Honest feedback:** distinguish saved, loading, failed, and offline states; do not imply unconfirmed work has been saved.

## Proposed visual foundations

These initial values are creative hypotheses to test, not established contrast-compliant pairs or final design tokens. Verify all text, icons, focus rings, and state combinations against applicable accessibility standards before adoption.

### Color

| Role | Candidate | Intended use |
| --- | --- | --- |
| Canvas | `#F7F8F5` | Soft neutral page background |
| Surface | `#FFFFFF` | Cards, dialogs, and input surfaces |
| Primary text | `#202B29` | Headings and body text |
| Secondary text | `#53635E` | Supporting information |
| Primary accent | `#2F6B5A` | Primary actions and selected states |
| Border | `#D8E2DA` | Subtle separation and input borders |
| Warning | `#8A5B16` | Caution, with icon and explanatory text |
| Destructive | `#A33232` | Destructive actions, with explicit labels |

Explore a small set of distinguishable Space accents, such as muted teal, blue, terracotta, and violet. Always pair an accent with the Space name; color is never the only indicator of identity, status, or permission. Define accessible hover, pressed, focus, disabled, and dark-surface combinations during validation.

### Typography

- Explore a friendly, highly legible sans-serif with good multilingual coverage; use the platform's system sans-serif as a safe fallback. No final typeface is selected.
- Starting scale: body 16 px, supporting text 14 px, section headings 20–24 px, and page headings 28–32 px. Avoid reducing essential labels to decorative caption sizes.
- Support browser zoom and text resizing without clipping or loss of functionality.

### Space, shape, and motion

- Starting spacing scale: 4, 8, 12, 16, 24, 32, and 48 px. Keep touch targets comfortably sized and test with real devices.
- Use soft, consistent corners and restrained elevation to separate interactive surfaces. Avoid shadows or decoration that compete with planning content.
- Keep transitions short and purposeful; respect reduced-motion preferences.

## Layout and navigation

### Mobile

- Show the current Space or All Spaces context prominently. Make switching context clear without losing the user's place.
- Provide direct access to Calendar, Tasks, and Places, plus a discoverable path to notifications and settings. The exact navigation pattern is a prototype decision, not a fixed requirement.
- Use focused creation flows with clearly labeled actions; ensure forms remain usable with the on-screen keyboard and browser zoom.
- Places always belong to the currently selected Space; All Spaces applies to Calendar and Tasks only.

### Desktop

- Explore a persistent navigation rail or sidebar and a wider content area. Use additional space for filters or secondary context when helpful.
- Preserve the same language and permissions as mobile; avoid a layout that is simply a stretched phone screen.

## Component patterns

| Component | Initial guidance |
| --- | --- |
| Space selector and badge | Show the Space name and optional accent; explicitly label All Spaces. |
| Navigation | Indicate the current destination with text and a visual state; support keyboard and screen-reader navigation. |
| Buttons | Distinguish primary, secondary, and destructive actions; labels explain the outcome. Provide focus, disabled, loading, and error feedback. |
| Forms | Persistent labels, contextual help, inline validation, and clear save/cancel actions; avoid relying on placeholders as labels. |
| Calendar event | Show title, date/time or all-day status, and Space identity where combined. Expose timezone and recurrence details when relevant. |
| Task row | Show title, completion status, optional assignee and due date, plus Space identity in the combined view. Completion controls must have accessible labels. |
| Reminder controls | Support multiple reminders; distinguish task-configured timing from recipient defaults and personal overrides. |
| Place card and map pin | Show place name, category, and available address; keep list access available if the map fails. Never rely on the pin color alone. |
| Member and invite controls | Display role names in plain language. Default invitation role to Viewer and make link approval, expiry, and revocation understandable. |
| Dialogs | Explain the consequence before sharing existing content or deleting a Space. Manage focus and allow a clear cancel path. |
| Notifications | Identify the relevant event and Space, with a path to the related content where applicable. Offer channel settings and per-Space mute. |
| Empty, loading, and error states | Explain what is happening, what remains available, and the next useful action; never suggest an unsaved change succeeded. |

## Required prototype states

- First-time onboarding and empty personal Space.
- All Spaces calendar and Tasks with clear labels and filters.
- Viewer versus Editor controls and Owner membership management.
- Invitation by email and join-link approval, plus the warning when sharing a personal Space.
- Timed/all-day event creation and simple recurrence with occurrence-versus-series choices.
- Task creation with optional assignee, due date/time, and multiple reminders.
- Manual place creation, import preview, and import failure with manual fallback; categorized list, map, and map-unavailable fallback.
- In-app notifications and preferences; offline and failure feedback.
- Delete confirmation and seven-day recovery explanation, including invalidated invitations and links.

## Validation and open decisions

Review proposed foundations and flows against the design brief on a narrow mobile viewport and a desktop viewport. Check text and non-text contrast, visible focus, keyboard navigation, screen-reader labels, resizing, touch targets, and reduced motion. Involve actual users when a beta cohort is available.

Final palette, Space accents, typography, component measurements, breakpoints, navigation, brand identity, and applicable accessibility targets remain open. The prototype may inform these decisions but does not settle them automatically.
