use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::{postgres::PgPoolOptions, sqlite::SqliteConnectOptions, PgPool, SqlitePool};
use std::path::Path;
use std::str::FromStr;
use uuid::Uuid;

use crate::models::Dataset;

const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS datasets (
    id TEXT PRIMARY KEY,
    share_token TEXT NOT NULL UNIQUE,
    file_name TEXT NOT NULL,
    columns_json TEXT NOT NULL,
    rows_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_datasets_share_token ON datasets(share_token);
"#;

/// A single storage abstraction backed by either SQLite (the zero-dependency
/// default, good for a single-replica deployment with a PVC) or Postgres
/// (for when you want to scale the API horizontally against a DB you
/// already run). Selected at startup by the scheme of DATABASE_URL.
#[derive(Clone)]
pub enum Store {
    Sqlite(SqlitePool),
    Postgres(PgPool),
}

impl Store {
    pub async fn connect(database_url: &str) -> Result<Self> {
        if let Some(path) = database_url.strip_prefix("sqlite://") {
            if let Some(parent) = Path::new(path).parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)
                        .with_context(|| format!("creating sqlite data dir {parent:?}"))?;
                }
            }
            let opts = SqliteConnectOptions::from_str(database_url)?
                .create_if_missing(true)
                .foreign_keys(true);
            let pool = SqlitePool::connect_with(opts)
                .await
                .context("connecting to sqlite database")?;
            let store = Store::Sqlite(pool);
            store.init_schema().await?;
            Ok(store)
        } else if database_url.starts_with("postgres://") || database_url.starts_with("postgresql://")
        {
            let pool = PgPoolOptions::new()
                .max_connections(10)
                .connect(database_url)
                .await
                .context("connecting to postgres database")?;
            let store = Store::Postgres(pool);
            store.init_schema().await?;
            Ok(store)
        } else {
            anyhow::bail!(
                "DATABASE_URL must start with sqlite:// or postgres://, got: {database_url}"
            )
        }
    }

    async fn init_schema(&self) -> Result<()> {
        match self {
            Store::Sqlite(pool) => {
                for stmt in SCHEMA_SQL.split(';').map(str::trim).filter(|s| !s.is_empty()) {
                    sqlx::query(stmt).execute(pool).await?;
                }
            }
            Store::Postgres(pool) => {
                for stmt in SCHEMA_SQL.split(';').map(str::trim).filter(|s| !s.is_empty()) {
                    sqlx::query(stmt).execute(pool).await?;
                }
            }
        }
        Ok(())
    }

    pub async fn create_dataset(
        &self,
        file_name: &str,
        columns: &serde_json::Value,
        rows: &serde_json::Value,
    ) -> Result<Dataset> {
        let id = Uuid::new_v4().to_string();
        let share_token = nanoid::nanoid!(12);
        let now = Utc::now().to_rfc3339();
        let columns_json = serde_json::to_string(columns)?;
        let rows_json = serde_json::to_string(rows)?;

        match self {
            Store::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO datasets (id, share_token, file_name, columns_json, rows_json, created_at, updated_at)
                     VALUES (?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(&id)
                .bind(&share_token)
                .bind(file_name)
                .bind(&columns_json)
                .bind(&rows_json)
                .bind(&now)
                .bind(&now)
                .execute(pool)
                .await?;
            }
            Store::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO datasets (id, share_token, file_name, columns_json, rows_json, created_at, updated_at)
                     VALUES ($1, $2, $3, $4, $5, $6, $7)",
                )
                .bind(&id)
                .bind(&share_token)
                .bind(file_name)
                .bind(&columns_json)
                .bind(&rows_json)
                .bind(&now)
                .bind(&now)
                .execute(pool)
                .await?;
            }
        }

        Ok(Dataset {
            id,
            share_token,
            file_name: file_name.to_string(),
            columns: columns.clone(),
            rows: rows.clone(),
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub async fn get_by_token(&self, token: &str) -> Result<Option<Dataset>> {
        let row: Option<(String, String, String, String, String, String, String)> = match self {
            Store::Sqlite(pool) => sqlx::query_as(
                "SELECT id, share_token, file_name, columns_json, rows_json, created_at, updated_at
                 FROM datasets WHERE share_token = ?",
            )
            .bind(token)
            .fetch_optional(pool)
            .await?,
            Store::Postgres(pool) => sqlx::query_as(
                "SELECT id, share_token, file_name, columns_json, rows_json, created_at, updated_at
                 FROM datasets WHERE share_token = $1",
            )
            .bind(token)
            .fetch_optional(pool)
            .await?,
        };

        match row {
            None => Ok(None),
            Some((id, share_token, file_name, columns_json, rows_json, created_at, updated_at)) => {
                Ok(Some(Dataset {
                    id,
                    share_token,
                    file_name,
                    columns: serde_json::from_str(&columns_json)?,
                    rows: serde_json::from_str(&rows_json)?,
                    created_at,
                    updated_at,
                }))
            }
        }
    }

    pub async fn update_by_token(
        &self,
        token: &str,
        columns: &serde_json::Value,
        rows: &serde_json::Value,
    ) -> Result<Option<Dataset>> {
        let now = Utc::now().to_rfc3339();
        let columns_json = serde_json::to_string(columns)?;
        let rows_json = serde_json::to_string(rows)?;

        let affected = match self {
            Store::Sqlite(pool) => {
                sqlx::query(
                    "UPDATE datasets SET columns_json = ?, rows_json = ?, updated_at = ? WHERE share_token = ?",
                )
                .bind(&columns_json)
                .bind(&rows_json)
                .bind(&now)
                .bind(token)
                .execute(pool)
                .await?
                .rows_affected()
            }
            Store::Postgres(pool) => {
                sqlx::query(
                    "UPDATE datasets SET columns_json = $1, rows_json = $2, updated_at = $3 WHERE share_token = $4",
                )
                .bind(&columns_json)
                .bind(&rows_json)
                .bind(&now)
                .bind(token)
                .execute(pool)
                .await?
                .rows_affected()
            }
        };

        if affected == 0 {
            return Ok(None);
        }
        self.get_by_token(token).await
    }
}
