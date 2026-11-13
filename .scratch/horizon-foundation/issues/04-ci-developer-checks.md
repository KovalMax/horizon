# 04 — Run developer checks and GitHub Actions

Status: implemented and verified locally; GitHub-hosted run pending
Blocked by: 03

Provide repeatable containerized formatting, static-analysis, and test commands for both applications, and configure GitHub Actions to run equivalent checks with PostgreSQL available to integration tests.

## Acceptance criteria

- The documented commands work without Node, Rust, or PostgreSQL installed on the host.
- CI runs on pushes and pull requests, provisions required test dependencies, and reports failures for failed checks.
- Document what runs locally versus in CI and how to troubleshoot failures; do not assume repository secrets or create a remote.
- Record any checks that cannot be exercised until the user creates and connects the GitHub repository.
