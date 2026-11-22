# Performance baseline

Run `make perf-baseline` from the repository root. The harness starts a
separate internal Docker project with release-mode Rust, the production web
image, PostgreSQL with `pg_stat_statements`, Mailpit, and HTTPS. It publishes
no host ports, changes no local CA trust, and never reads the development or
production database. It generates synthetic data only and removes its
containers and database on exit.

For a short plumbing check:

```sh
PERF_SAMPLES=5 PERF_MIXED_SECONDS=0 make perf-baseline
```

Use the default run for comparison: 100 measured requests per read workload
after three warmups (four concurrent), up to 12 measured writes per fan-out
workload, and a 65-second mixed-read window spanning at least one interval of
the minute-based reminder worker. Use consistent Docker CPU/memory settings
and avoid other workloads on the same Docker VM. For a shared runner baseline,
use the manual **Performance baseline (report only)** GitHub workflow after
it reaches the default branch. No CI threshold is enforced.

## Measurements

- **Typical:** 4 shared Spaces, 400 Tasks, 80 Calendar events, 240 Saved
  places, 400 Notifications and 5 other recipients in the first Space.
- **Large:** 24 shared Spaces, 12,480 Tasks, 2,880 Calendar events, 5,760
  Saved places, 15,000 Notifications and 100 other recipients in the first
  Space. A separate 32-Space cohort adds data outside both accounts’
  memberships. Repeating event data includes moved and cancelled overrides.
- Use real Mailpit sign-in, then record per-route request counts for three
  signed-in All Spaces reloads in Chromium. `toNetworkIdleMs` includes
  Playwright's idle window; it is not a web vital.
- Time authenticated HTTP requests from a separate client inside Docker,
  directly to the API. Numbers include reading the response but not HTTPS
  proxy time or browser rendering. Compare Calendar's 7- and 90-day ranges
  (including a deeper page), first/deep To-dos, Saved places and
  Notifications pages, and Calendar event creation with Notification
  fan-out. The fan-out sample is small, so p95/p99 are descriptive only.
- Record whole-run PostgreSQL calls, rows, execution time and buffer counts,
  plus actual execution plans extracted from the API’s SQL. Write plans run
  only on disposable data and roll back. Query times are PostgreSQL execution
  time, not total API or pool-wait time.
- Time Calendar range phases separately: database reads, recurrence generation,
  moved-override handling, sorting, and page preparation. This adds opt-in
  numeric API logs only (`HORIZON_PROFILE_CALENDAR_RANGE=1` on the benchmark
  API). They are disabled in normal environments and carry no identifiers.
- Report mixed reads while the real Task reminder worker is enabled and
  record cumulative worker statement calls. There is no worker-off control in
  this first baseline; don’t attribute a latency change to the worker alone.

Results default to `.local/perf-baseline/run.*/` for local runs. Set
`HORIZON_PERF_RESULTS` to choose another root; GitHub Actions uses
`$RUNNER_TEMP/horizon/performance`. Start with `summary.md`,
use `baseline.json` for comparisons, then inspect `postgres.json`,
`query-plans.txt` and `fixtures.txt`. Reports exclude tokens, query bind values
from HTTP traffic, and Mailpit bodies. Query plans include only reproducible
synthetic parameters. Compare multiple runs of the same setup before setting
a threshold. The first deterministic gate should target request counts, not
timings.
