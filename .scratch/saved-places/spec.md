# Saved Places persistence

Status: Implemented.

## Outcome

Authenticated Members can browse manual Saved places in the Spaces they can access. Owners and Editors can save a place; Viewers can read but not write. Signed-out sample places remain session-only.

## Contract

- Persist a Space-scoped name, built-in category, address, and optional note.
- Reads are membership-scoped; Owners and Editors may write; Viewers may not.
- Custom categories, contact details, map-provider integration, directions, and Maps-link import are deferred.
- To-dos remain persisted; other sample planner data remains session-only.

## Test seams

- Authenticated HTTP against real PostgreSQL verifies persistence, Space scope, and role authorization.
- React Places workflow verifies API loading and saving through visible Places screens and forms.
