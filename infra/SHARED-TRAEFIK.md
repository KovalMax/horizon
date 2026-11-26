# Shared production Traefik contract

Horizon does not run a production Traefik container. Copy the standalone edge
`compose.yaml` and `Makefile` to `~/traefik/` on the VM. That deployment owns
host ports 80/443, TLS certificates, and a Docker network named `public-edge`.
Horizon's production `web` and `api` services join that external network using
the unique aliases `horizon-web` and `horizon-api`.

The Horizon API routes share a Traefik per-client-IP token bucket: 120 requests
per minute, with a burst of 30. The same router covers both the same-origin
`/api` path and the direct API hostname, so switching hosts does not reset the
allowance. Traefik stores this limiter in memory per instance; if the edge is
scaled to multiple Traefik instances, configure distributed rate limiting with
Redis or the instances will enforce independent buckets.

Horizon also limits sign-in requests in PostgreSQL. The Horizon API router
removes RFC 7239 `Forwarded`, and the API accepts only a single
`X-Forwarded-For` address from the exact `AUTH_TRUSTED_PROXY_IP` peer
(production Compose defaults to Traefik's `172.30.0.250`). The API uses the
socket peer for every other connection and for absent or malformed proxy
metadata. Keep insecure forwarded-header trust off and keep the API port
unpublished. This policy assumes the shared Traefik is the direct public
edge and replaces incoming `X-Forwarded-For`; review it before adding an
upstream proxy/CDN or moving Traefik to another address. Do not trust the
entire shared subnet.

The edge Traefik file provider must watch the host directory
`~/traefik/dynamic/`, mounted read-only into the Traefik container as a
directory. This repository owns `infra/traefik/dynamic.yml`; its release
workflow atomically installs it as `~/traefik/dynamic/horizon.yml`. Do not
replace the directory contents during a Horizon deployment: Zwei's route file
is managed independently as `zwei.yml`.

Configure the edge Traefik with the `web` and `websecure` entrypoints, an
HTTP-to-HTTPS redirect, and a certificate resolver named `letsencrypt`. The
Horizon routers use that resolver. The shared network must resolve the unique
service aliases to Horizon's web container on port 8080 and API container on
port 8000. The edge deployment should create the external network before
Horizon is deployed. The coordinated Zwei configuration currently expects the
network subnet `172.30.0.0/24` and Traefik's fixed address `172.30.0.250`; check
that the subnet does not overlap the VM's existing Docker networks before
creating it. Zwei's trusted-proxy CIDR must match that fixed address.

## First migration from the per-project proxy

Use the same VM account for the edge Compose project and the release workflows,
so `~/traefik/dynamic/` resolves to the same directory. Use a planned
maintenance window. Before deploying this Compose configuration:

1. Configure the standalone edge Compose project to create `public-edge` and
   mount `~/traefik/dynamic/` into Traefik's watched file-provider directory.
2. Ensure the proposed `172.30.0.0/24` subnet does not overlap existing VM
   networks and configure Traefik at `172.30.0.250`, matching Zwei's
   `TRUSTED_PROXY_CIDRS` setting.
3. Stage this repository's route file and Zwei's route file in the watched
   directory. Verify the static Traefik configuration and KYC credential mount.
4. From Zwei's current deployment directory, stop and remove only its old
   Traefik service while the old Compose file still defines it:
   `docker compose stop traefik && docker compose rm -f traefik`.
5. Start the central edge project so it claims ports 80/443. Routes can return
   502 until the apps move from `zwei_edge` to `public-edge`.
6. Deploy the updated Zwei and Horizon stacks, then verify public endpoints,
   certificates, and KYC Basic Auth. Do not stop or remove app/database
   containers or volumes as part of the proxy cutover.

The deployment workflow only installs Horizon's route file and updates
Horizon-owned services; it does not start, stop, or reconfigure the shared edge
proxy. The deploy account must be able to create and atomically replace files in
`~/traefik/dynamic/`.
