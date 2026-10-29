//! Request/response shapes for the JSON API and the stored dataset record.
//!
//! `columns` and `rows` are deliberately kept as opaque `serde_json::Value`s:
//! the server stores and returns whatever the frontend sends without
//! validating it. The only place the server looks inside them is the CSV
//! export in `routes::export_csv`, which expects `columns` to be an array of
//! objects with a `name` field and `rows` to be an array of objects keyed by
//! column name. See `docs/api.md` for the shape the frontend uses.

use serde::{Deserialize, Serialize};

/// A stored dataset, as held in the `datasets` table.
///
/// `created_at` and `updated_at` are RFC 3339 timestamps (UTC) stored as
/// text so the same schema works on SQLite and Postgres.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Dataset {
    /// Internal UUIDv4 primary key. Returned on create but not used by any
    /// other endpoint.
    pub id: String,
    /// 12-character nanoid. Knowing it grants full read/write access.
    pub share_token: String,
    /// Original upload file name, used to name the CSV export.
    pub file_name: String,
    /// Column definitions, stored verbatim.
    pub columns: serde_json::Value,
    /// Row data, stored verbatim.
    pub rows: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
}

/// Body of `POST /api/datasets`.
#[derive(Debug, Deserialize)]
pub struct CreateDatasetRequest {
    pub file_name: String,
    pub columns: serde_json::Value,
    pub rows: serde_json::Value,
}

/// Body of `PUT /api/d/:token`. Replaces `columns` and `rows` wholesale;
/// `file_name` cannot be changed after creation.
#[derive(Debug, Deserialize)]
pub struct UpdateDatasetRequest {
    pub columns: serde_json::Value,
    pub rows: serde_json::Value,
}

/// Response to `POST /api/datasets`.
#[derive(Debug, Serialize)]
pub struct CreateDatasetResponse {
    pub id: String,
    pub share_token: String,
}

/// Response to `GET` and `PUT /api/d/:token`. Omits `id`, `share_token`
/// and `created_at`.
#[derive(Debug, Serialize)]
pub struct DatasetResponse {
    pub file_name: String,
    pub columns: serde_json::Value,
    pub rows: serde_json::Value,
    pub updated_at: String,
}

impl From<Dataset> for DatasetResponse {
    fn from(d: Dataset) -> Self {
        DatasetResponse {
            file_name: d.file_name,
            columns: d.columns,
            rows: d.rows,
            updated_at: d.updated_at,
        }
    }
}
