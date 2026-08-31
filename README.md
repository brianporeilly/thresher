# Sift server

A tiny self-hosted backend for the Sift CSV-triage tool. Single Rust binary,
embeds the frontend, talks to SQLite (default) or Postgres.

## Layout

```
src/
  main.rs          entry point, routing, graceful shutdown
  store.rs         Store enum over SqlitePool / PgPool — one table, JSON blobs
  routes.rs        POST /api/datasets, GET/PUT /api/d/:token, GET .../export.csv
  static_files.rs  serves the embedded frontend, SPA-style fallback to index.html
models.rs          request/response shapes
static/
  index.html       the frontend (same file that works standalone, extended
                    with hosted-mode: loads /d/:token from the API, autosaves
                    edits, shows a "Copy link" button)
  papaparse.min.js  vendored locally so nothing is fetched from a CDN
Dockerfile          multi-stage musl build -> distroless/static image
```

## Environment variables

| Var            | Default                   | Notes                                             |
|----------------|---------------------------|----------------------------------------------------|
| `DATABASE_URL` | `sqlite://data/sift.db`   | `sqlite://path/to/file.db` or `postgres://user:pass@host/db` |
| `PORT`         | `8080`                    | HTTP listen port                                   |
| `RUST_LOG`     | `sift_server=info,tower_http=info` | standard `tracing_subscriber` env filter  |

The SQLite path's parent directory is created automatically if missing, so
`DATABASE_URL=sqlite://data/sift.db` just works with a mounted volume at
`./data` (or a k8s PVC at that path).

## Running locally

```
cargo run
# -> http://localhost:8080
```

Against Postgres instead:

```
DATABASE_URL=postgres://user:pass@localhost:5432/sift cargo run
```

No migrations step needed — the one table is created with
`CREATE TABLE IF NOT EXISTS` on startup, identical DDL for both backends.

## API

- `POST /api/datasets` — body `{ "file_name": "...", "columns": [...], "rows": [...] }`
  → `{ "id": "...", "share_token": "..." }`
- `GET /api/d/:token` — → `{ "file_name", "columns", "rows", "updated_at" }`
- `PUT /api/d/:token` — body `{ "columns": [...], "rows": [...] }`, same response shape
- `GET /api/d/:token/export.csv` — server-rendered CSV download
- `GET /healthz` — liveness check, returns `ok`

Anyone with a `share_token` has full read/write on that dataset — it's a
capability link, like an unlisted doc, not a login. There's no user/auth
system in front of it.

## Concurrency

Whole-document replace, last-write-wins. Fine for a couple of people sharing
a link and toggling things; not a CRDT, so two people editing the exact same
row at the exact same moment will have one edit clobber the other.

## Scaling notes

- **SQLite**: single writer. Run exactly one replica with a PVC mounted at
  the `DATABASE_URL` path. Don't scale this past 1 pod.
- **Postgres**: stateless binary, scale replicas freely, point at whatever
  Postgres you already run.

## Building the image

```
docker build -t sift-server .
docker run -p 8080:8080 -v $(pwd)/data:/data sift-server
```

## A build note from development

This was scaffolded and code-reviewed in a sandbox pinned to Rust 1.75
(Dec 2023), which is old enough that several transitive dependencies'
latest patch releases now require the `edition2024` Cargo feature (needs
Cargo 1.85+). That forced some ugly exact-version pins during development.
**None of that should apply to you** — with any current Rust toolchain
(1.85+, which `rustup` or a recent `rust:slim`/`rust:1` image will give you),
`cargo build` against the `Cargo.toml` here should resolve normally with no
pinning needed. If you're on an older pinned toolchain for some reason and
hit `edition2024` errors, the fix is: find which crate's latest version
requires it (`cargo tree -i <crate>@<version>` after a failed
`cargo check` will show you), and pin that one dependency down a minor
version in `Cargo.toml`.
