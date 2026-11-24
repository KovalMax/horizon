# Horizon browser checks

Run from the repository root using Docker:

| Command | Coverage |
| --- | --- |
| `make test-browser` | Deterministic Playwright suite covering sign-in errors, rate limits, callback cleanup, session restoration, and an axe scan of mobile sign-in. |
| `make test-e2e-stack` | Real HTTPS browser journeys: Mailpit delivery and sign-in, mobile Task persistence and keyboard focus, sign-out, and Owner/Editor/Viewer collaboration. |

Both targets run in PR and main CI, and must pass on the release tag before
publishing images. They use their own Compose projects and
do not need or modify the local development stack. The signed-in suite starts
PostgreSQL on disposable storage, Mailpit, the Actix API, a production web
build and an HTTPS proxy. The browser accesses `https://horizon.localhost/`
inside that network; neither the database nor Mailpit is published to the host.
The browser-only proxy does not apply the development rate limit, so the
multi-user test can run from one IP. Rate-limit feedback remains covered by the
deterministic suite.

Configure the PR check **Quality / Required** in the `main` branch ruleset
after its first successful run if merges must be blocked on these checks.

The stack generates a temporary CA without changing the host trust store or
development certificates. Test accounts use unique `example.test` addresses
and all email stays inside Mailpit. Signed-in tests have screenshots, traces,
and videos disabled so that callback and join links are not uploaded. The
deterministic suite can retain traces under `.local/browser-results/` (or
`HORIZON_BROWSER_RESULTS`). GitHub Actions uses `$RUNNER_TEMP/horizon/browser`
and uploads only deterministic failure traces.

For a manual review, start the development stack with `make up`, then open
`https://horizon.localhost/` through the browser or Playwright MCP. Mailpit's
development inbox is `http://localhost:8025`.

The service worker registers only in production builds. The signed-in stack
checks the manifest and verifies that an offline launch shows a public
fallback, without caching Settings, API data, or sign-in callbacks.
To check iPhone installation on a deployed HTTPS build, open Horizon in
Safari, choose **Share → Add to Home Screen**, and launch it from the Home
Screen. Installability and the offline message do not imply push delivery
or offline access to saved plans.

If the sign-in link opens Safari instead of the installed app, request a
sign-in email from the Home Screen app, then enter the eight-digit code from
that email in the same app. Use either the code or the link, not both.
