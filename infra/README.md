# Horizon infrastructure

Run local commands from the repository root with `make`. The development
targets keep the repository root as the Compose project directory so moving
the manifests does not rename the running project or its volumes.

| Manifest | Purpose |
| --- | --- |
| `compose.dev.yaml` | Local development stack (`make up`, `make ps`, `make down`) |
| `compose.checks.yaml` | Isolated API and web checks (`make check`) |
| `compose.browser.yaml` | Deterministic and signed-in Playwright suites |
| `compose.perf.yaml` | Release-mode override for `make perf-baseline` |
| `compose.prod.yaml` | Production services deployed by the release workflow |

Dockerfiles and their per-image ignore rules live in `docker/api/` and
`docker/web/`. Their build contexts are still `apps/api/` and `apps/web/`;
Dockerfile `COPY` paths are relative to those contexts.

`scripts/` contains the shell entry points used by the root Makefile.
[`perf/`](perf/README.md) contains the benchmark runner, SQL fixtures, and
reporting code.

See [the shared production Traefik contract](SHARED-TRAEFIK.md) before changing
edge routing or deployment.
