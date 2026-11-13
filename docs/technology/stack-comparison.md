# Horizon MVP: initial technology comparison

Status: React with TypeScript is selected for web; Rust/Actix and PostgreSQL are the agreed preferred backend baseline. The slices have not been benchmarked. Horizon is a mobile-first, responsive web beta; native apps are later. The existing prototypes are design and discovery artifacts, not a production architecture.

## Working recommendation

Use **React with TypeScript for the web client**, alongside **Rust/Actix and PostgreSQL as the preferred backend baseline**. React Native is a possible future mobile choice, not a commitment; shared TypeScript or logic does not guarantee shared UI or integrations. The React slice was closer to the preferred visual direction at the effort invested, but the slices are not evidence of a performance or accessibility advantage.

These preferences are not evidence that one option is universally faster, cheaper, or more accessible. Deployment constraints and measured results can change them. Redis remains an option, not a beta dependency: browser/CDN caching may suffice for static assets; only introduce Redis for a demonstrated need such as shared rate limiting or short-lived state, with explicit cache invalidation and authorization boundaries.

Use **Docker and Docker Compose** for local development. Deploy beta production to a **virtual machine over SSH**, running a production Compose file. Keep local and beta configurations appropriate to their environments; choose the VM provider and establish secrets handling, TLS, persistent database storage, backups and restore testing, migrations, monitoring, updates, and recovery before deployment. Compose simplifies service management but does not supply those operational safeguards by itself. Redis should only be included if its need is established.

## Client comparison

| Criterion | Flutter web | Web-native client, e.g. React/TypeScript |
| --- | --- | --- |
| Later mobile apps | Potential to share Dart, UI, and application logic across web and native clients; platform-specific work still exists. | Responsive web is direct; separate native apps later would require an additional client and potentially different UI code. |
| Browser fit | Flutter targets interactive applications and renders using browser graphics APIs; assess accessibility, input behavior, startup, and plugin compatibility on Horizon's supported devices. | Uses web platform primitives and established browser tooling; accessibility is not automatic and still needs implementation and testing. |
| Maps and integrations | Evaluate actual Flutter web plugin support and behavior; do not assume mobile plugin features also work on web. | Browser map SDK integrations are a natural fit, but still depend on provider support, terms, and costs. |
| Team and operations | One language/framework across targets may reduce duplication if the team knows Dart and later builds Flutter apps. | Browser-oriented skills and tooling may make the initial web beta easier for a web-experienced team. |

Flutter describes its web support as suitable for app-centric experiences, including single-page apps, while distinguishing text-heavy, document-oriented websites. Its documentation also notes that web output differs from a traditional document-centric web model. Horizon's authenticated planner is app-centric; marketing/help pages can be evaluated separately. See [Flutter web overview](https://docs.flutter.dev/platform-integration/web) and [web FAQ](https://docs.flutter.dev/platform-integration/web/faq).

## Backend comparison

| Option | Potential fit | Trade-offs to validate |
| --- | --- | --- |
| Rust with Actix Web | Matches the stated preference; suitable candidate for a typed API and background work. | Team Rust experience, implementation speed, library choices, deployment, observability, and operational support matter more than assumed performance. |
| Rust with another framework, e.g. Axum | Retains Rust while offering another ecosystem choice. | Compare framework familiarity, middleware and testing support; do not switch solely on popularity. |
| A team-familiar alternative, e.g. TypeScript or Go | Could reduce delivery risk if the team already has relevant experience. | Different type/runtime and operational trade-offs; compare with an actual small slice rather than assuming faster delivery. |

Whichever backend is chosen needs authorization at Space boundaries, secure authentication, persistence, scheduled reminder delivery, and reliable handling of email and provider failures. A separate service or specific architecture is not yet required.

## Developer experience: preliminary assessment

These are documentation-based observations, not measurements of the slices. The current experiments have no authored comparison tests and both concentrate UI and state in a single file, so neither demonstrates a maintainable production architecture. The React experiment currently uses JSX rather than TypeScript and its install reported dependency vulnerabilities; neither point is inherent to React.

| Concern | React web / potential React Native | Flutter web / potential Flutter mobile |
| --- | --- | --- |
| Automated tests | Unit and component tests typically require selecting and configuring tools such as Vitest and Testing Library; Playwright can cover browser journeys. Test behavior through accessible interfaces, not implementation details. | Flutter supplies unit, widget, and integration testing facilities. Widget tests are convenient for UI behavior but do not replace real-browser, platform, or accessibility checks. |
| Reuse | React components are reusable on web. React Native shares the React model and can share TypeScript and platform-independent logic, but DOM/CSS components and native UI generally need adaptation; libraries may differ by platform. | Widgets and Dart logic can often be reused across web and mobile; responsive layouts, plugins, browser behavior, and native integrations still need platform-specific work and verification. |
| Maintainability | Flexible ecosystem with more choices about routing, state, testing, and dependencies; those choices require consistent conventions and dependency review. | A more integrated SDK and widget system reduce some initial choices, but Dart/Flutter expertise, plugin compatibility, SDK upgrades, and separation of concerns still matter. |
| Configuration | With Vite, public `VITE_` values are normally substituted at build time; browser runtime configuration requires a separately served public config or endpoint. | `--dart-define` supplies compile-time declarations. Flutter web also needs a separately served public configuration or endpoint if one image must run unchanged across environments. |

For either client, **anything sent to the browser is public**: never put database credentials, email-provider secrets, private API keys, or signing secrets in client configuration. Docker Compose `.env` interpolation, container environment variables, build arguments, and browser-visible configuration are distinct mechanisms; setting an environment variable on a static web-server container does not automatically modify an already-built client. A runtime configuration file or endpoint must expose an explicit allowlist of non-secret values, be correctly escaped and cached, and be tested independently. A same-origin `/api` route may avoid needing a configurable API origin; decide after deployment design. Keep backend secrets server-side and plan VM secret handling separately.

The disposable slices now have three matching behavior tests: create and complete a Task, open/close People, and expand/collapse the calendar. Both suites passed in Docker. React uses Vitest, Testing Library, jsdom, and a dialog API shim; Flutter uses widget tests. These are not equivalent to real-browser or screen-reader tests, and no controlled setup-effort or performance comparison has been performed. Test runtime config with two different non-secret values using the same built image; that experiment remains outstanding. The React test dependency install reported six vulnerabilities, including two critical; investigate and update before any use beyond the isolated experiment.

Sources: [Flutter testing overview](https://docs.flutter.dev/testing/overview), [React components](https://react.dev/learn/your-first-component), [Vite environment variables](https://vite.dev/guide/env-and-mode), and [Dart compilation environment declarations](https://dart.dev/libraries/core/environment-declarations). Verify details against the versions ultimately selected.

## Database comparison

| Criterion | PostgreSQL | MySQL |
| --- | --- | --- |
| Core relational model | Suitable for users, Space membership, events, Tasks, and Places with transactional integrity. | Also suitable for the core relational model with transactional integrity. |
| Horizon considerations | A reasonable default when team experience and hosting are equal; evaluate timezone handling, indexing, backups, and any eventual spatial requirements. | A sound choice when the team or hosting environment favors it; evaluate the same requirements rather than assuming missing capabilities. |
| Decision | Preferred backend baseline. | Viable alternative if requirements change. |

Neither database handles recurrence, reminder delivery, or authorization correctly by itself: those behaviors require explicit design and tests.

## Integration feasibility checks

1. **Maps-link import:** Collect representative user-provided long and short links with consent; determine which documented, permitted paths can identify a place and retrieve the desired fields. Verify redirect handling, provider terms, attribution, retention rules, quotas, prices, privacy, and failure behavior. Never assume arbitrary links can be reliably parsed. Manual entry remains the fallback. Avoid building a fetcher that follows arbitrary user-supplied URLs without SSRF protections.
2. **Directions handoff:** Google documents cross-platform Maps URLs for launching searches and directions without an API key; this is an outbound handoff, not evidence that arbitrary inbound shared links can be resolved or imported. See [Google Maps URLs](https://developers.google.com/maps/documentation/urls/get-started).
3. **Map display:** Evaluate an actual provider and web SDK for mobile and desktop browser support, accessibility, key restrictions, attribution, usage costs, and graceful list fallback.
4. **Magic-link email:** Compare managed identity and self-managed options for single-use, expiring links; safe redirects; verification; email delivery; account recovery; rate limiting; and beta invitation handling. Check behavior when email security scanners open links.
5. **Email notifications and reminders:** Evaluate transactional email delivery, retries, duplicates, unsubscribe/preference handling, and scheduling across timezones and daylight-saving transitions. Decide acceptable delivery guarantees; do not assume exactly-once delivery.

## Proposed decision gates

1. Confirm team skills, hosting preferences, budget, target browsers, and accessibility expectations.
2. Build equivalent **throwaway client slices**: a Space screen with responsive navigation, accessible form/dialog, a task, and a map placeholder or permitted provider integration. Compare on real mobile Safari/Chrome and desktop, including keyboard/screen-reader use, load behavior, and maintainability. Set measurable acceptance thresholds before testing.
3. Run a small, permitted place-import investigation with representative links, including failure and manual fallback; record coverage, terms, and estimated usage cost. Do not treat an unsupported scrape as a solution.
4. Compare email identity and delivery options and a minimal Rust/API/database path, including a permission test and scheduled-reminder scenario.
5. Review remaining technical results with the product owner and record an ADR only for decisions whose context and trade-offs merit one. React/TypeScript for web has been selected; the native mobile technology remains open. No implementation-ready work is implied by this draft.

## Client comparison: proposed acceptance criteria

Build equivalent, disposable Flutter web and React/TypeScript slices using fictional local data and the same representative design. Do not build production authentication, provider integrations, or full product features for this comparison. Assess both against the same device/browser versions and record conditions, results, blockers, and workarounds.

1. **Equivalent scope:** switch between All Spaces and a Space; display a Space's Tasks; create and complete a Task in a dialog; expand a calendar or Places component. Both slices must behave equivalently, including empty and validation states.
2. **Responsive behavior:** at 360 px and 1280 px viewport widths, core flows must be usable without horizontal page scrolling or clipped controls; test on real mobile Safari and mobile Chrome when available, plus desktop browsers. Record devices and OS/browser versions.
3. **Accessibility:** operate navigation, creation, and expansion with keyboard only, including visible focus and dialog focus return; inspect semantic labels and reading order with VoiceOver and at least one desktop screen reader where available. Check contrast and 200% text/browser zoom; record issues rather than declaring compliance from automated checks alone.
4. **Performance:** measure cold and warm startup and transfer size for equivalent production builds under the same device and network conditions. Record distributions, not a single anecdotal run. Numeric pass thresholds require agreement before measurement; don't infer a winner from unmeasured expectations.
5. **Mobile path and maintainability:** document what code and libraries can actually be shared with Flutter mobile versus React Native, what UI and integrations would need platform-specific work, and estimated maintenance complexity. Evaluate map integration and magic-link deep-link handling separately; a placeholder is not proof of feasibility.
6. **Decision:** React/TypeScript for web has been selected based on product direction and developer experience, not a controlled performance or accessibility comparison. Neither slice becomes production code by default. Seek explicit approval before beginning implementation.
