use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;

use crate::models::{CreateDatasetRequest, CreateDatasetResponse, DatasetResponse, UpdateDatasetRequest};
use crate::store::Store;

pub struct AppError {
    status: StatusCode,
    message: String,
}

impl AppError {
    fn not_found(msg: &str) -> Self {
        Self { status: StatusCode::NOT_FOUND, message: msg.to_string() }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        if self.status == StatusCode::INTERNAL_SERVER_ERROR {
            tracing::error!("{}", self.message);
        }
        (self.status, self.message).into_response()
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        Self { status: StatusCode::INTERNAL_SERVER_ERROR, message: format!("internal error: {e}") }
    }
}

pub async fn create_dataset(
    State(store): State<Arc<Store>>,
    Json(payload): Json<CreateDatasetRequest>,
) -> Result<Json<CreateDatasetResponse>, AppError> {
    let ds = store
        .create_dataset(&payload.file_name, &payload.columns, &payload.rows)
        .await?;
    Ok(Json(CreateDatasetResponse { id: ds.id, share_token: ds.share_token }))
}

pub async fn get_dataset(
    State(store): State<Arc<Store>>,
    Path(token): Path<String>,
) -> Result<Json<DatasetResponse>, AppError> {
    let ds = store
        .get_by_token(&token)
        .await?
        .ok_or_else(|| AppError::not_found("dataset not found"))?;
    Ok(Json(ds.into()))
}

pub async fn update_dataset(
    State(store): State<Arc<Store>>,
    Path(token): Path<String>,
    Json(payload): Json<UpdateDatasetRequest>,
) -> Result<Json<DatasetResponse>, AppError> {
    let ds = store
        .update_by_token(&token, &payload.columns, &payload.rows)
        .await?
        .ok_or_else(|| AppError::not_found("dataset not found"))?;
    Ok(Json(ds.into()))
}

pub async fn export_csv(
    State(store): State<Arc<Store>>,
    Path(token): Path<String>,
) -> Result<Response, AppError> {
    let ds = store
        .get_by_token(&token)
        .await?
        .ok_or_else(|| AppError::not_found("dataset not found"))?;

    let headers: Vec<String> = ds
        .columns
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|c| c.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
        .collect();

    let rows = ds.rows.as_array().cloned().unwrap_or_default();

    let mut wtr = csv::WriterBuilder::new().from_writer(vec![]);
    wtr.write_record(&headers)
        .map_err(|e| anyhow::anyhow!("writing csv header: {e}"))?;
    for row in &rows {
        let record: Vec<String> = headers
            .iter()
            .map(|h| match row.get(h) {
                Some(serde_json::Value::String(s)) => s.clone(),
                Some(serde_json::Value::Null) | None => String::new(),
                Some(other) => other.to_string(),
            })
            .collect();
        wtr.write_record(&record)
            .map_err(|e| anyhow::anyhow!("writing csv row: {e}"))?;
    }
    let data = wtr
        .into_inner()
        .map_err(|e| anyhow::anyhow!("finalizing csv: {e}"))?;

    let filename = format!("{}-threshed.csv", ds.file_name.trim_end_matches(".csv"));
    let disposition = format!("attachment; filename=\"{filename}\"");

    Ok((
        [
            ("content-type", "text/csv; charset=utf-8"),
            ("content-disposition", disposition.as_str()),
        ],
        data,
    )
        .into_response())
}
