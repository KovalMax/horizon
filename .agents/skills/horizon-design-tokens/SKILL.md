---
name: horizon-design-tokens
description: Use for Horizon typography, semantic CSS tokens, color contrast, spacing, elevation, and identity accents.
---

# Horizon design tokens

Read `apps/web/src/styles.css` for live values. Use semantic variables rather
than copying literal colors into components.

| Token | Use |
| --- | --- |
| `--ink`, `--muted` | Main and secondary text |
| `--primary` | Main actions and focus outlines |
| `--canvas`, `--border`, `--control` | App background, dividers, and form control outlines |
| `--personal`, `--home`, `--trip` and their `-soft` variants | Space identity accents and surfaces |
| `--sage`, `--sage-soft` | Positive accent and muted positive surface |
| `--radius`, `--shadow`, `--ease-standard` | 14px base radius, subtle elevation, and standard motion |

- Use locally bundled Figtree (400–800) and existing component spacing.
  Prefer white cards, restrained outlines/shadows and quiet surfaces.
- Keep role/Space labels and control state visible without color alone.
  Verify contrast in real rendered combinations, including focus, muted
  text, inactive states and soft accent pairs. Respect reduced motion.
- Reuse components and variants before adding tokens. Add a CSS variable
  when several consumers share a semantic role, not as a new name for a
  one-off literal. For layout sizes, icon rules and component states, use
  [horizon-design](../horizon-design/SKILL.md).
