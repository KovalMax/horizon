---
name: horizon-mailpit
description: Use for Horizon SMTP configuration, local magic-link and invitation delivery, and Mailpit-backed tests.
---

# Horizon Mailpit

- Local stack: `make up`, then `https://horizon.localhost/welcome`.
  The SMTP destination inside Compose is `mailpit:1025`; the host inbox is
  `http://localhost:8025`. Use synthetic `example.test` recipients. Mailpit
  is not the production SMTP provider.
- Automated browser and perf stacks do not publish the inbox or use existing
  mail. The test client polls `http://mailpit:8025/api/v1/search` by unique
  recipient and reads the matching message; filter by subject. Open only
  local callback links. Avoid dumping message bodies, one-time links, or
  session tokens to logs, screenshots, traces, or exported artifacts.
- If the inbox is unavailable, verify service health and host port binding
  separately. A running container with only `8025/tcp` listed may have no
  host mapping. Port 8025 conflicts can prevent the whole stack from starting;
  do not regenerate HTTPS certificates for an SMTP/inbox problem.
- Keep real delivery tests in `make test-e2e-stack`; use the deterministic
  suite for injected retry and rate-limit failures. Unmodified test stacks
  allow several roles on one IP without the dev edge throttle; that does not
  mean production or development lacks rate limiting.
