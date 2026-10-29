# Contributing

## Prerequisites

- A current stable Rust toolchain (1.85 or newer, see the build note in
  the README). Nothing else is needed for the default SQLite backend.
- Docker, if you want to build the image.
- A Postgres instance, if you want to test the Postgres backend.

## Dev loop

```
cargo run
# -> http://localhost:8080, data in ./data/thresher.db
```

Debug builds read `static/` from disk, so you can edit `static/index.html`
and just refresh the browser. Rust changes need a restart. To start over
with an empty database, delete `./data/`.

Against Postgres:

```
docker run --rm -d --name thresher-pg -e POSTGRES_PASSWORD=pass -e POSTGRES_DB=thresher -p 5432:5432 postgres:16
DATABASE_URL=postgres://postgres:pass@localhost:5432/thresher cargo run
```

Quick API smoke test:

```
tok=$(curl -s localhost:8080/api/datasets -H 'content-type: application/json' \
  -d '{"file_name":"t.csv","columns":[{"name":"a","type":"text","options":[]}],"rows":[{"a":"1"}]}' \
  | sed 's/.*"share_token":"\([^"]*\)".*/\1/')
curl -s localhost:8080/api/d/$tok
curl -s localhost:8080/api/d/$tok/export.csv
```

## Before you push

CI (`.github/workflows/ci.yml`) runs these checks, and any warning fails
the build:

```
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features
cargo build --locked --release
docker build -t thresher .
```

Commit `Cargo.lock` changes along with the dependency change that caused
them, because CI builds with `--locked`.

## Where things go

- New endpoint: add the handler in `src/routes.rs`, add its shapes to
  `src/models.rs`, and register the route in `src/main.rs`. Document it in
  `docs/api.md`.
- New query: add it to `src/store.rs` with **both** a SQLite (`?`) and a
  Postgres (`$n`) version. Keep the DDL portable between them.
- Frontend: everything is in `static/index.html`, with no build step. The
  page doesn't load anything from third-party hosts, so put any new
  library in `static/` rather than loading it from a CDN.

## Releasing

See "Cutting a release" in the README: push a `vX.Y.Z` tag.
