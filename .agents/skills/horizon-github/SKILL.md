---
name: horizon-github
description: Use for Horizon pull requests, stacked merges, GitHub Actions checks, artifact uploads, and release workflows.
---

# Horizon GitHub

- PRs to `main` and pushes to `main` call `validate.yml`. The shared
  `browser.yml` runs deterministic and signed-in jobs; the release workflow
  calls it for the tag before publishing. `Quality / Required` is the
  aggregate PR check, not proof that a branch ruleset enforces it. Verify
  current rules and run status in GitHub; do not change settings without scope.
- `api.yml` reuses one Docker build for formatting, compilation, tests and
  Clippy; release runs also check fresh and repeat migrations on disposable
  PostgreSQL. Signed-in browser checks reuse the same AMD64 BuildKit cache.
  The optimized ARM64 production build has a separate, release-only GHCR
  cache; never deploy the cache tag.
- `.github/workflows/performance.yml` is manual and report-only. CI artifacts
  go to `$RUNNER_TEMP/horizon`, not `.local/`. Upload only named reports and
  deterministic browser traces; signed-in traces, callback URLs and join
  links must stay out of artifacts. Do not opt in to broad hidden-file uploads.
- Before resolving stacked PRs, compare files/trees as well as ancestry:
  squash merges can preserve every file while changing commit IDs. Prefer
  ordinary merges when updating a published branch, preserve existing work,
  and verify the final diff and exact tested SHA. A stacked PR not targeting
  `main` will not trigger the normal PR workflow.
- Passing local checks is not passing remote CI; starting a deploy is not
  confirming readiness. Use `gh pr view`, `gh pr checks`, and `gh run view`
  to verify status. Do not merge or deploy just because a PR is green.
