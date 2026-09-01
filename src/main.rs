mod models;
mod routes;
mod static_files;
mod store;

use axum::{
    routing::{get, post},
    Router,
};
use std::{net::SocketAddr, sync::Arc};
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};

use store::Store;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "thresher=info,tower_http=info".into()),
        )
        .init();

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://data/thresher.db".to_string());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let store = Arc::new(Store::connect(&database_url).await?);
    tracing::info!(
        backend = match *store {
            Store::Sqlite(_) => "sqlite",
            Store::Postgres(_) => "postgres",
        },
        "connected to store"
    );

    let api = Router::new()
        .route("/datasets", post(routes::create_dataset))
        .route(
            "/d/:token",
            get(routes::get_dataset).put(routes::update_dataset),
        )
        .route("/d/:token/export.csv", get(routes::export_csv))
        .with_state(store);

    let app = Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .nest("/api", api)
        .fallback(static_files::static_handler)
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutting down");
}
