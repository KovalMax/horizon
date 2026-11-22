---
name: horizon-architecture
description: Use to design Horizon feature boundaries, place business rules, or change cross-layer contracts.
---

# Horizon architecture

- Horizon is one product context. Use `GLOSSARY.md` in requirements, code, UI,
  tests, and docs. A Space owns each Calendar event, To-do list and Saved place;
  Tasks belong to a To-do list. Each Space has one Owner. Editors manage
  shared content but not membership; Viewers read only.
- Keep the API authoritative for permissions and persistence. A hidden
  control is not an authorization check. All Spaces combines Calendar and
  To-dos; it does not turn Saved places into a global collection.
- Organize by feature first. Keep HTTP/React presentation, use-case workflows,
  domain decisions, and persistence separate where those boundaries carry
  meaning. Prefer cohesive concrete modules over speculative interfaces or
  one-file-per-type scaffolding. Add an adapter or contract only for a real
  dependency or variation, such as the mail transport.
- Keep invariants near their owners, DTOs at transport boundaries when their
  shape or privacy differs, and mappers/factories for nontrivial rules rather
  than one-line wrappers. Check the contract on both sides before changing it.
- Check existing ADRs if present. Record durable decisions, not implementation
  minutiae. Keep local specs and open questions in the scratch location from
  `AGENTS.md`. Move legacy inline tests only when touching their area.
