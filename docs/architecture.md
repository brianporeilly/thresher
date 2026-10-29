# Architecture

Thresher is a single Rust binary. It serves an embedded single-page frontend
and a small JSON API, and stores everything in one database table.

```
browser ──HTTP──▶ axum router (src/main.rs)
                   ├─ /healthz              → "ok"
                   ├─ /api/*                → src/routes.rs ──▶ Store (src/store.rs) ──▶ SQLite | Postgres
                   └─ everything else       → src/static_files.rs → embedded static/*
                                               (unknown paths → index.html)
```

Every response goes through gzip compression, permissive CORS, and
request tracing (`tower-http` layers in `main.rs`).

## Components

### `src/main.rs`: startup and routing

1. Set up `tracing` using `RUST_LOG` (default `thresher=info,tower_http=info`).
2. Read `DATABASE_URL` (default `sqlite://data/thresher.db`) and `PORT`
   (default `8080`). If `PORT` isn't a valid number, it falls back to 8080
   without an error.
3. `Store::connect`, which also creates the schema.
4. Build the router and listen on `0.0.0.0:$PORT`.
5. On SIGINT or SIGTERM, stop accepting connections, let in-flight requests
   finish, and exit.

### `src/store.rs`: persistence

`Store` is an enum with a `Sqlite(SqlitePool)` variant and a
`Postgres(PgPool)` variant. The `DATABASE_URL` scheme picks which one:

| Scheme | Behaviour |
|--------|-----------|
| `sqlite://<path>` | Creates the parent directory and the file if they don't exist. A relative path (`sqlite://data/x.db`) is resolved against the working directory. An absolute path needs three slashes (`sqlite:///data/x.db`, which the Docker image uses). |
| `postgres://`, `postgresql://` | Pool of up to 10 connections. The database must already exist. |

Each method runs the same query on either backend. The only difference is
the placeholder syntax (`?` vs `$n`). Queries are checked at runtime, not
at compile time with `sqlx::query!`, so building doesn't need a database.

#### Schema

```sql
CREATE TABLE IF NOT EXISTS datasets (
    id           TEXT PRIMARY KEY,         -- UUIDv4
    share_token  TEXT NOT NULL UNIQUE,     -- 12-char nanoid
    file_name    TEXT NOT NULL,
    columns_json TEXT NOT NULL,            -- JSON text
    rows_json    TEXT NOT NULL,            -- JSON text
    created_at   TEXT NOT NULL,            -- RFC 3339, UTC
    updated_at   TEXT NOT NULL             -- RFC 3339, UTC
);
CREATE INDEX IF NOT EXISTS idx_datasets_share_token ON datasets(share_token);
```

All columns are `TEXT` so the same DDL works on both backends. The schema
is applied with `IF NOT EXISTS` on every startup, and there is no
migrations framework. If you ever need to change the schema, you'll have
to add a migration mechanism first. `CREATE TABLE IF NOT EXISTS` won't
change a table that already exists.

### `src/routes.rs`: handlers

These are thin wrappers that call the `Store` and map `None` to `404` and
errors to `500`. See [api.md](api.md) for the request and response
contract. `export_csv` is the only handler that looks inside `columns` and
`rows`.

### `src/models.rs`: shapes

These are the Serde structs for requests, responses, and the stored record.
`columns` and `rows` are `serde_json::Value`, so the server isn't tied to
the frontend's data model.

### `src/static_files.rs` and `static/`: frontend

`rust-embed` compiles `static/` into the binary. Release builds serve the
embedded copy, so you have to rebuild to ship frontend changes. Debug
builds (`cargo run`) read the files from disk, so you can edit
`static/index.html` and just refresh the page.

If a path doesn't match an embedded file, the server returns `index.html`.
That lets `/d/<token>` work as a client-side route.

`static/index.html` is the whole app: plain JavaScript with no build step
and no framework. It uses PapaParse (`static/papaparse.min.js`, kept in the
repo so nothing loads from a CDN) to parse CSVs in the browser.

## Frontend flow

The app has three screens in one page: **landing** (upload), **column
config**, and **main** (table on desktop, cards on mobile).

1. **Upload.** PapaParse reads the CSV in the browser. All values are
   strings. Each column is set to `segmented` (choice buttons) if its name
   matches `keep|drop|status|decision|watch|verdict` or it has 1–4 distinct
   non-empty values. Otherwise it's `text`.
2. **Configure.** The user can switch each column between Text and Choice
   buttons and edit the option labels. The first label gets the `keep`
   colour, the second `drop`, and the rest `neutral`.
3. **Start.** The first time, the frontend sends `POST /api/datasets`. If
   that works, the URL changes to `/d/<token>` and a "Copy link" button
   appears. If it fails (no server, or a body over the size limit), the app
   keeps working in **local-only mode** with no error shown, and the user
   saves their work with "Export CSV".
4. **Triage.** Clicking a choice button writes its `label` into the row.
   Clicking the selected button again clears it. Search, per-column
   filters (including "(undecided)"), and sorting all happen in the
   browser. Sorting is numeric when both values parse as numbers.
5. **Autosave.** In hosted mode, each change starts a 700 ms timer, and
   when it fires the frontend sends `PUT /api/d/<token>` with the full
   `columns` and `rows`. If the save fails, it retries every 3 s until one
   succeeds.
6. **Shared link.** When the page loads at `/d/<token>`, the frontend
   fetches `GET /api/d/<token>` and opens straight to the main screen. If
   the token is unknown, it shows an error on the landing page and
   resets the URL to `/`.

Open tabs don't sync with each other. A tab only sees other people's edits
after a reload, and its next save overwrites theirs (last write wins). This
is fine for a couple of people taking turns on a link. It isn't built for
several people editing at the same time.

### Running the frontend without the backend

`index.html` loads PapaParse from the absolute path `/papaparse.min.js`, so
the page must be served over HTTP with `static/` as the web root, for
example `cd static && python3 -m http.server`. Opening the file directly
(`file://`) won't work because PapaParse doesn't load. Served without the
API, the app runs in local-only mode as described above.

## Deployment shapes

- **SQLite:** one replica with a persistent volume at the database path.
  SQLite allows only one writer, and two replicas would each need their
  own file anyway, so don't scale past one.
- **Postgres:** the binary keeps no state, so you can run as many replicas
  as you like behind a load balancer. `/healthz` works as a liveness probe.
  It doesn't check the database, so don't use it as a readiness check for
  database connectivity.

The Docker image is a fully static musl binary on
`distroless/static-debian12:nonroot` (uid/gid 65532). It defaults to
`DATABASE_URL=sqlite:///data/thresher.db` and declares `/data` as a volume.

## CI and releases

- `.github/workflows/ci.yml` runs on pushes to `main` and on PRs. It
  checks `cargo fmt --check`, runs `cargo clippy -D warnings`,
  `cargo test`, and a release build, and builds the Docker image (without
  pushing).
- `.github/workflows/release.yml` runs on `v*.*.*` tags. It builds musl
  binaries for x86_64 and aarch64 on native runners and attaches them, with
  SHA-256 checksums, to a GitHub Release. It also builds the image for each
  architecture natively and pushes a multi-arch manifest to
  `ghcr.io/<owner>/<repo>` tagged `vX.Y.Z`, `vX.Y`, `vX`, and `latest`.
