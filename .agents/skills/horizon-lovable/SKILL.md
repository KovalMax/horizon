---
name: horizon-lovable
description: Use when comparing Horizon with the Lovable reference or assessing which prototype behavior belongs in the app.
---

# Horizon Lovable reference

Reference: [max-horizon.lovable.app](https://max-horizon.lovable.app/).
Implementation: `https://horizon.localhost/`.

1. Compare the same route and state at desktop and 390px mobile, with 320px
   checks for dense layouts. Inspect opened menus/dialogs, empty/error
   states, and keyboard behavior, not only the first screenshot.
2. Treat the reference as visual and interaction guidance, not the product
   spec. Keep Horizon's persisted behavior, roles, and terminology. Sample
   content, Review guide, functional maps/geocoding, and unsupported reminder
   controls do not become requirements via design comparison.
3. Keep the existing React/Vite implementation and shared UI; do not replace
   the stack with generated prototype code. Follow [horizon-design](../horizon-design/SKILL.md)
   and [horizon-design-tokens](../horizon-design-tokens/SKILL.md).
4. Default to read-only reference inspection. Editing, accepting drafts,
   deploying, or sending build messages to Lovable is separate work and
   requires authorization. Do not copy real member data into the prototype.
