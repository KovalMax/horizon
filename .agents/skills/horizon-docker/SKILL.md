---
name: horizon-docker
description: Use for local Docker Compose lifecycle, Makefile commands, development HTTPS, and port troubleshooting.
---

# Horizon Docker

- Use `make help`, `make up`, `make ps`, `make logs SERVICE=api`,
  `make restart`, `make migrate`, and `make down`. `make down` preserves the
  PostgreSQL volume. Never remove volumes or stop unrelated projects as a
  troubleshooting shortcut.
- The root `Makefile` selects `infra/compose.dev.yaml` with the repository root
  as its project directory, preserving the local project name and volumes.
  Compose manifests live in `infra/`; Dockerfiles and their ignore rules live
  in `infra/docker/`, with app-specific build contexts. Local Traefik routes
  `https://horizon.localhost/` and `https://api.horizon.localhost/`.
  Use `https://horizon.localhost/api/health/ready` for a same-origin API and
  database readiness check. An initial 502 during rebuild is not a TLS
  diagnosis; confirm readiness.
- If TLS fails, check `make ps`, host listeners on ports 443/8025, and the
  certificate actually served with SNI `horizon.localhost`. `TRAEFIK DEFAULT
  CERT` indicates the expected certificate is not being served; check the
  proxy, routing, mounts and competing containers before touching trust.
- `make certs` replaces the local CA and leaf certificates. Use it when
  necessary, not on every restart. Trust only the intended local CA and
  restart the proxy after certificate changes. Never commit private keys.
- `make test-api` requires the running dev stack. `make check` provisions an
  isolated PostgreSQL; browser and perf targets also use isolated Compose
  projects and do not need the dev stack. CI builds/releases use different
  Compose inputs; see [horizon-infra](../horizon-infra/SKILL.md).
