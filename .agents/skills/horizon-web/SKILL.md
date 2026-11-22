---
name: horizon-web
description: Use for React, TypeScript, Vite, planner workflows, route state, and frontend API calls.
---

# Horizon web

- Start in `apps/web/src/features/`, not `App.tsx` by default. Auth lives in
  `features/auth/`; planner pages, workflows and API modules live in
  `features/planner/`. Use feature hooks/functions for workflows, API modules
  for requests, and `shared/ui.tsx` or existing planner components for stable
  reusable presentation. Keep pages declarative and components independent of
  URL and network mechanics. This is Vite, not Next.js.
- Use `authenticatedFetch` through feature API modules for protected requests.
  Keep transport shapes distinct from UI models where they differ and pass an
  abort signal or ignore stale responses when a route, Space, or filter changes.
- Derive values during render. Prefer functional updates, lazy expensive
  initialization, stable keys, immutable updates and module-scope components.
  Separate hidden or costly subtrees when useful; measure before introducing
  memoization, bundle splitting, or list windowing. Use passive input listeners
  only when they never call `preventDefault`.
- Keep All Spaces startup reads limited; fetch Owner management on Members,
  Recently deleted in Settings, and Members needed for Task assignment on demand.
  Keep invitation visibility and the unread badge. Preserve a selected
  assignee if its list is temporarily unavailable.
- Filter Saved places by Space and category before pagination, preserve full
  Space category totals, and expose selected state. Preserve loading, empty,
  error, retry and permission distinctions; do not label a 403 as offline.
- Verify with `make test-web` and the relevant browser target; for UI changes
  also read [horizon-design](../horizon-design/SKILL.md).
