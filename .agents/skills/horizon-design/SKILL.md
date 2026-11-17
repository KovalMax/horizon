---
name: Horizon UI design
description: Use when changing Horizon's visual design, interaction patterns, responsive layouts, components, icons, or UI copy.
---

# Horizon UI design

Use this skill for every user-interface change in Horizon. Treat the current web
app and `https://max-horizon.lovable.app` as one product family: preserve
working behavior while matching the prototype's visual system and responsive
interaction patterns.

## Before editing

1. Read the root `GLOSSARY.md` and use its canonical product terms (Space,
   Member, Owner, Editor, Viewer, Calendar event, To-do list, Task, Saved place,
   Place category, All Spaces view, Notification).
2. Check `docs/adr/` for relevant decisions when present. Do not silently
   override documented product decisions.
3. Inspect the target route in the local app and the prototype at desktop and
   mobile sizes. Inspect relevant dialogs, menus, empty/loading/error states,
   and role-specific behavior—not just the first viewport.
4. Keep Horizon's React + TypeScript + Vite architecture. Keep page components
   declarative and place workflows in existing feature hooks/API modules.

The prototype's **Review guide** is a review aid for the human evaluating the
prototype, not a Horizon product feature. Do not add it to the app's routes,
navigation, or UI.

## Visual foundation

The reference uses **Figtree**, a warm near-white canvas, subdued green-gray
text, fine sage borders, rounded white cards, and restrained shadows. The
canonical CSS variables live in `apps/web/src/styles.css`; extend or reuse
those variables rather than introducing near-duplicate colors.

Current palette and semantic roles:

| Token | Value | Use |
| --- | --- | --- |
| `--ink` | `oklch(27.8% 0.016 182.8)` | Main text |
| `--muted` | `oklch(48.5% 0.021 174.5)` | Secondary text |
| `--primary` | `oklch(48.3% 0.069 172.2)` | Primary action and focus |
| `--canvas` | `oklch(97.8% 0.004 121.6)` | App background |
| `--border` | `oklch(90.3% 0.015 151.8)` | Dividers and outlines |
| `--control` | `oklch(61.9% 0.021 171.1)` | Form control borders |
| `--personal` / `--personal-soft` | `oklch(49% 0.044 266.9)` / `oklch(95.7% 0.01 266)` | Personal Space identity |
| `--home` / `--home-soft` | `oklch(53.7% 0.097 44.6)` / `oklch(95.5% 0.015 51.3)` | Household Space identity |
| `--trip` / `--trip-soft` | `oklch(50.1% 0.086 238.5)` / `oklch(95.4% 0.013 234.8)` | Trip Space identity |
| `--sage` / `--sage-soft` | `oklch(48.3% 0.069 150)` / `oklch(95% 0.015 150)` | General positive/sage accent |

The base radius is `14px`; use the existing card, pill, and control radii
according to their component roles. Use `--shadow` for low-emphasis elevation.
Favor whitespace and subtle borders over heavy shadows or strong gradients.

### Dialog and control scale

- Keep desktop create/edit dialogs content-sized and compact; do not use
  viewport-wide widths for ordinary forms. The Task dialog should cap near
  `760px`, with a `24px` heading, `14px` labels, `48px` inputs, and restrained
  field gaps. Calendar event dialogs are narrower still.
- Keep dialogs bounded by the viewport and scroll internally only when their
  content requires it. On mobile, use the existing full-width bottom-sheet
  treatment with compact horizontal padding.
- To-do completion controls should be `24px` circles, not `32px`. Use a clear
  muted outline and subtle shadow when open; completed Tasks use a filled
  primary circle with a centered white checkmark.
- Prefer compact icon actions for To-do list rename/delete: pencil for rename
  and a red trash icon for delete. Icon-only actions must retain descriptive
  accessible names and visible focus.

## Layout and responsive behavior

- Desktop sidebar: approximately `288px`; main content max width: `1024px`,
  centered in the remaining column, with `40px` horizontal and top padding.
- The sidebar collapses at the existing `850px` breakpoint. At mobile widths,
  the sticky top bar and fixed five-item bottom navigation replace desktop
  navigation.
- Mobile main content uses `16px` horizontal gutters and `20px` top padding;
  account for the bottom navigation and safe-area insets.
- Preserve the existing `1100px` intermediate breakpoint and inspect layouts
  just above and below breakpoints when changing grids or navigation.
- The To-dos view is a two-column grid on wider screens, with an inline dashed
  **New list** card; collapse to one column on mobile.
- Saved places use a vertical list, dynamic category counts, and compact
  actions. Keep the disabled Map affordance only; functional map support was
  explicitly deferred.
- Prevent horizontal page overflow at 390px and at the minimum supported width
  (320px). Horizontal scrolling is acceptable only for intentionally
  scrollable controls, such as filter pills or cards.

## Components, icons, and interaction

- Reuse `Button`, `Icon`, `SpaceGlyph`, `PageHeader`, and existing feature
  components from `apps/web/src/shared/ui.tsx` and planner components.
- Use the existing inline SVG `Icon` names; do not add emoji or mix icon
  libraries for a one-off visual.
- Primary actions use the filled green pill button. Secondary actions use
  outline or quiet variants. Keep one clear primary action per region; avoid
  duplicate actions between the header and empty state.
- Keep Space identity visible in labels and use the semantic personal,
  household, and trip colors consistently.
- Use native controls or accessible equivalents. Give icon-only controls
  accessible names, preserve visible keyboard focus, use buttons for actions
  and links for navigation, and retain correct dialog semantics.
- Custom comboboxes and popovers must close on both Escape and an outside
  pointer interaction. When auditing a picker, open it, click a sibling control
  or page surface outside the picker, and confirm the menu disappears without
  dismissing its containing dialog.
- Desktop dialogs are centered cards; on small screens they become bottom
  sheets. Preserve bounded height, internal scrolling, close/cancel actions,
  backdrop behavior, and Escape handling.
- Keep transitions short and purposeful. Respect `prefers-reduced-motion`;
  do not animate layout or data in ways that make the interface harder to use.
- Confirm destructive operations and explain the impact. Keep loading, empty,
  error, retry, disabled, and success-feedback states explicit.

## Product-specific constraints

- Do not change business permissions to make a screen look closer. Viewers can
  read shared content but cannot edit it; Editors manage shared content but not
  membership; Owners manage membership and ownership.
- Saved places are scoped to exactly one Space and use built-in categories.
  Custom categories are deferred.
- Do not add functional map/geocoding behavior unless the user explicitly
  reopens that decision.
- Prefer the canonical terminology in UI copy and accessible names. In
  particular, use **To-dos**, **To-do list**, **Task**, **Saved places**, and
  **Space** consistently.

## Validation checklist

1. Run the web tests and production build (`npm test`, `npm run build` from
   `apps/web`).
2. Run relevant API tests and `cargo fmt --check` when API/Rust code changes.
3. Use Playwright MCP to inspect changed routes at a desktop viewport and at
   390px mobile; check 320px when controls are dense.
4. Inspect interaction states: open and outside-dismissed pickers, menus,
   dialogs, forms, focus, success/error, empty/loading states, and role-specific
   affordances.
5. Verify there is no page-level horizontal overflow and that labels,
   navigation, focus indicators, and touch targets remain usable.
6. Report unavailable dependencies (such as a database) separately from code
   failures; do not claim a blocked test passed.
