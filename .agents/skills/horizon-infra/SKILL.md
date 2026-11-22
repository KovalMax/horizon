---
name: horizon-infra
description: Use for Horizon production Compose, Traefik route configuration, security boundaries, deployments, and rollback.
---

# Horizon infrastructure

- Read `infra/compose.prod.yaml`, `infra/traefik/dynamic.yml` and
  `infra/SHARED-TRAEFIK.md`. Production uses a shared Traefik edge, not the
  dev Traefik container. Only Horizon's route file is managed by Horizon.
  Preserve neighboring apps' routes and the `public-edge` aliases
  `horizon-web` and `horizon-api`.
- Keep PostgreSQL on the internal database network and unexposed to the host.
  Production web serves the Vite build via unprivileged Nginx (8080); the
  Actix API listens on 8000. Keep same-origin `/api/` stripping and the
  supported direct API hostname aligned with CORS/TLS/rate-limit configs.
- Read `.github/workflows/release.yml` before changing rollout behavior.
  Tagged ARM64 images are published to GHCR. Deployment stages config and
  image tag, migrates first, rolls services, then checks readiness. The
  previous image can be restored; an applied migration is still applied.
- Preserve least privilege and read-only containers where configured. Keep
  deployment secrets in the deployment environment or GitHub secrets, never
  in tracked config or artifacts. Verify a health route only establishes
  readiness, not a user journey. Do not deploy or alter the shared edge as
  part of local testing.
