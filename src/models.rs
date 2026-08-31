use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Dataset {
    pub id: String,
    pub share_token: String,
    pub file_name: String,
    pub columns: serde_json::Value,
    pub rows: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateDatasetRequest {
    pub file_name: String,
    pub columns: serde_json::Value,
    pub rows: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDatasetRequest {
    pub columns: serde_json::Value,
    pub rows: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct CreateDatasetResponse {
    pub id: String,
    pub share_token: String,
}

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
