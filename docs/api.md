# HTTP API reference

All API routes live under `/api` and send and receive JSON, except the CSV
export. Error bodies are plain text, not JSON.

There is no authentication. A dataset's `share_token` is a capability:
anyone who has it can read and overwrite that dataset. See
[Security model](#security-model).

## Data model

The server stores `columns` and `rows` as opaque JSON and does **not**
validate them. Any JSON value is accepted and returned unchanged. The shape
below is what the bundled frontend reads and writes. The server's CSV export
also relies on it, so other clients should stick to it.

### `columns`

An ordered array of column definitions:

```json
[
  { "name": "Title",  "type": "text",      "options": [] },
  { "name": "Status", "type": "segmented", "options": [
      { "label": "Keep",  "kind": "keep" },
      { "label": "Drop",  "kind": "drop" },
      { "label": "Maybe", "kind": "neutral" }
  ] }
]
```

| Field     | Meaning |
|-----------|---------|
| `name`    | Column header. Also the key used to look up the value in each row. |
| `type`    | `"text"` (shown read-only) or `"segmented"` (shown as a row of choice buttons). |
| `options` | For `segmented` columns, the buttons to show. `label` is the value written into the row when you pick it. `kind` (`keep`, `drop`, `neutral`) only sets the colour. Empty for `text` columns. |

### `rows`

An array of flat objects keyed by column `name`. Values are strings as
parsed from the uploaded CSV. An empty string (or a missing key) in a
`segmented` column means "undecided".

```json
[
  { "Title": "First item",  "Status": "Keep" },
  { "Title": "Second item", "Status": "" }
]
```

## Endpoints

### `POST /api/datasets`

Create a dataset and get its share token.

Request:

```json
{ "file_name": "items.csv", "columns": [ ... ], "rows": [ ... ] }
```

Response `200`:

```json
{ "id": "0b6c3c1e-...-uuid", "share_token": "V1StGXR8_Z5j" }
```

`id` is an internal UUID. No other endpoint uses it. `share_token` is a
12-character [nanoid](https://github.com/ai/nanoid) (`A-Za-z0-9_-`). The
shareable link the frontend builds is `/d/<share_token>`.

### `GET /api/d/:token`

Fetch a dataset.

Response `200`:

```json
{
  "file_name": "items.csv",
  "columns": [ ... ],
  "rows": [ ... ],
  "updated_at": "2024-05-01T12:34:56.789012345+00:00"
}
```

`updated_at` is an RFC 3339 UTC timestamp.

### `PUT /api/d/:token`

Replace a dataset's `columns` and `rows`. `file_name` can't be changed.

Request:

```json
{ "columns": [ ... ], "rows": [ ... ] }
```

The response is `200` with the same shape as `GET`, showing the new
`updated_at`.

This replaces the whole document and the last write wins. There is no
version or `If-Match` check. If two clients save at about the same time,
the later save overwrites the earlier one completely, even if they changed
different rows.

### `GET /api/d/:token/export.csv`

Download the dataset as CSV (`text/csv; charset=utf-8`, sent as an
attachment named `<file_name without .csv>-threshed.csv`).

- The header row is the `name` of each entry in `columns`, in order.
  Entries without a string `name` are skipped.
- Each cell is `row[name]`. Strings are written as-is, `null` or missing
  values as an empty cell, and any other JSON value (number, boolean,
  object) as its JSON text.

The bundled frontend doesn't call this endpoint. Its "Export CSV" button
builds the file in the browser. The endpoint is there for scripts and
`curl`.

### `GET /healthz`

Liveness check. Always returns `200` with body `ok`. It doesn't touch the
database, so it can't tell you the database is reachable.

## Status codes

| Status | When |
|--------|------|
| `200`  | Success. |
| `400` / `415` / `422` | The request body isn't valid JSON, is missing `Content-Type: application/json`, or is missing a required field (`file_name`, `columns`, `rows`). These come from axum's `Json` extractor. |
| `404`  | Unknown share token (body: `dataset not found`). |
| `405`  | Known route, wrong method (e.g. `DELETE /api/d/:token`). |
| `413`  | Request body larger than the limit (see below). |
| `500`  | Database or serialization failure. The body is `internal error: <details>` and the error is also logged. |

Unknown paths, including unknown paths under `/api/`, aren't 404s. They
fall through to the frontend and return `index.html` with `200`.

## Limits

- **Request body size:** 2 MB. This is axum's default `Json` body limit and
  the app doesn't change it. A `POST` or `PUT` bigger than this gets a `413`.
  The JSON encoding of a CSV is larger than the CSV file itself, so the
  largest CSV you can share is somewhat under 2 MB. When creation fails, the
  frontend falls back to local-only mode without telling the user.
- **No delete endpoint.** Datasets are kept indefinitely. To remove one,
  delete its row from the `datasets` table by hand.

## Security model

- Share tokens are 12 random characters from a 64-character alphabet
  (about 72 bits). That is enough that tokens can't be guessed, so a token
  works like an unlisted link. Anyone you give it to, or who sees it in a
  URL, browser history, or log, has full read and write access.
- The server uses a permissive CORS policy (`CorsLayer::permissive()`), so
  JavaScript on any origin can call the API if it knows a token.
- There are no users, sessions, or rate limits. If you expose Thresher to
  the internet, put it behind something that handles those (a reverse
  proxy with auth, a VPN, and so on) if you need them.
