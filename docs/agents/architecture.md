# Architecture and code organization

Use domain-driven design (DDD) to keep product rules explicit, and SOLID as a practical guide for assigning responsibilities and managing dependencies. Apply these principles proportionally: prefer the simplest structure that protects a real boundary; do not create empty layers, one-file-per-type scaffolding, or abstractions without a current use.

Use the canonical terms in `GLOSSARY.md`. Keep unrelated product contexts separate, and make dependencies point toward domain and application rules rather than toward frameworks or infrastructure.

## Rust API

Organize code by feature or domain capability first, then by responsibility within that feature. Keep the HTTP, application, domain, and persistence boundaries explicit.

Typical responsibilities are:

- **Models:** domain entities, value objects, and domain errors. Keep invariants close to the model that owns them. Avoid putting Actix, SQLx, or serialization concerns in core domain types unless the boundary genuinely requires it.
- **Controllers:** thin HTTP handlers. Parse/validate transport input, call an application service, and translate its result to an HTTP response. Do not put business workflows or SQL queries in handlers.
- **Requests and responses:** transport DTOs for inbound and outbound HTTP. Keep them distinct from domain models when their validation, privacy, or representation differs.
- **Services:** application use cases coordinate domain behavior and ports. Use domain services only for domain rules that do not naturally belong to one model.
- **Repositories:** define persistence-facing contracts at the application/domain boundary and implement them in infrastructure. Keep SQL and database row mapping in the implementation; do not let persistence models become the domain model by accident.
- **Factories:** use when creation has meaningful rules or coordination. Simple constructors should stay with their model rather than gaining a factory layer by default.

These are responsibility names, not a checklist. Add only the files/modules a feature needs; split a responsibility when it has a useful independent boundary. A feature might use `models.rs`, `services.rs`, `controllers.rs`, and `repositories.rs`, or a deeper directory structure when it improves cohesion.

### Rust module roots

`mod.rs` is module wiring only: child-module declarations and, when useful, re-exports. Put no implementation, business rules, types, constants, handlers, or tests in it. Prefer the modern `feature.rs` plus `feature/` layout for new modules when it makes the tree clearer; apply the same wiring-only rule to any module root.

### Rust tests

- Put all Rust tests in separate files under `apps/api/tests/`; do not add `#[test]`, `#[tokio::test]`, `#[actix_web::test]`, or `#[cfg(test)] mod tests` to `src/` modules.
- Prefer integration tests that exercise public use cases and HTTP behavior. Keep test fixtures/helpers in the test tree (for example, `tests/support/`), not in production modules.
- If integration tests need a callable application boundary, expose a production-worthy library API (for example, through `src/lib.rs`) rather than adding test-only visibility or hooks to production code.
- Name test files after the feature or behavior, such as `tests/authentication.rs` or `tests/space_membership.rs`. Keep each test focused on observable behavior and include relevant failure/authorization cases.

## React web app

Organize by feature/domain capability, with UI and application responsibilities separated. Adapt the same boundaries to React rather than reproducing backend layers mechanically.

- **Routes and page components** compose a screen and connect it to feature hooks; keep them small and declarative.
- **Components** render UI and expose clear props. Keep reusable presentation components independent of routing and network details.
- **Hooks/application logic** coordinate user actions and screen state. Put business decisions in feature-level functions or hooks, not in large page components or generic UI controls.
- **Models/types and schemas** describe domain concepts and validate data at boundaries. Keep API request/response DTOs distinct where their shape differs from the UI/domain representation.
- **API/repository modules** own HTTP calls, serialization, and mapping at the boundary. Components should not issue ad hoc `fetch` calls or know persistence details.
- **Factories/mappers** are appropriate when they encapsulate non-trivial creation or transformation rules; avoid wrappers that only rename a one-line operation.

Group these files under a feature when that makes ownership clear (for example, `features/spaces/` with `components/`, `hooks/`, `api/`, and `types.ts`). Keep shared UI primitives in a shared area, and promote code there only when it is genuinely reused and has a stable interface. Avoid monolithic `App.tsx` or page components; use the app shell for global composition and keep feature workflows in feature modules.

### React tests

- Put web tests in separate files under `apps/web/tests/`, not alongside production files in `src/`.
- Prefer tests of user-visible behavior and feature workflows with Testing Library. Keep fixtures and test helpers under the test tree.
- Test through accessible roles, labels, and text rather than implementation details. Use focused unit tests for pure domain functions and integration-style tests for components/hooks working together.

Tests currently embedded in Rust or React source are legacy exceptions, not patterns to copy. Move them to the appropriate `tests/` tree when making a related, scoped change; do not broaden a feature task into an unrelated test-architecture rewrite.

## SOLID and dependency rules

- **Single responsibility:** a module should have one coherent reason to change.
- **Open/closed:** extend behavior behind a stable boundary when variation is expected; do not add speculative plugin systems.
- **Liskov substitution:** implementations of an interface must preserve its documented behavior and failure semantics.
- **Interface segregation:** keep ports and component props focused on the needs of their callers.
- **Dependency inversion:** business rules depend on small abstractions where a real external boundary exists; framework and database details implement those abstractions.

Do not introduce interfaces solely to satisfy a pattern. Prefer concrete, testable code until there is a meaningful boundary, then extract the smallest useful contract. When changing legacy code, keep the change scoped and migrate toward these rules incrementally; avoid unrelated rewrites.
