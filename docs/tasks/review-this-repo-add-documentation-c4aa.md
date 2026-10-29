# Task: review-this-repo-add-documentation-c4aa

## Goal

Review this repo (Thresher: Rust/axum CSV-triage server + embedded single-file frontend) and add documentation.

## Constraints

- No Rust toolchain in sandbox (rustup download 403) — cannot run cargo fmt/clippy/test locally. Rust edits limited to comments; avoid clippy doc lints (doc_lazy_continuation etc.) and trailing whitespace since CI runs clippy -D warnings + fmt --check.
- Docs scope: don't change behaviour; report bugs as findings instead of fixing.

## Done

- Rustdoc module/item comments added to all src/*.rs (comments only, no code changed).
- README: fixed GHCR tag names (v1.2/v1, not 1.2/1, per metadata-action patterns), fixed layout indentation, added Documentation section.
- New: docs/api.md (data shape, endpoints, status codes, 2 MB limit, security model), docs/architecture.md (components, schema, frontend flow, deployment, CI/release), CONTRIBUTING.md (dev loop, CI checks, where things go).
- Review findings recorded in PR body.

## Decisions

- Deliverables: rustdoc comments in src/*, README corrections, docs/api.md, docs/architecture.md, CONTRIBUTING.md. Review findings go in PR body (not fixed here).

## Open questions

- Not verified with cargo (no toolchain in sandbox); CI fmt/clippy will confirm the comment-only Rust changes.
- Review findings (unfixed, owner to triage): 2 MB body limit silently drops large CSVs to local-only mode; /api/* unknown paths return index.html 200; 500s leak internal error text; no delete endpoint; prerelease tags would move `latest`; README "standalone" claim needs HTTP serving (absolute /papaparse.min.js); no tests (CI `cargo test` is a no-op); `/healthz` doesn't check DB.

## Next step

Owner reviews PR; decide which review findings to turn into follow-up tasks.
