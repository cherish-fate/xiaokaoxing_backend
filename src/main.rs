mod app;
mod auth;
mod auth_ext;
mod config;
mod db;
mod response;
mod routes;
mod state;
mod utils;

use anyhow::Result;
use tokio::net::TcpListener;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;

use crate::{app::build_router, config::AppConfig, state::AppState};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    init_tracing();

    let config = AppConfig::from_env()?;

    let db_connection = db::connect(config.database_url.as_deref()).await?;
    let addr = config.addr()?;
    let state = AppState {
        config: std::sync::Arc::new(config),
        db: db_connection.as_ref().map(|c| c.pool.clone()),
        database_name: db_connection.map(|c| c.database_name),
    };

    let app = build_router(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let listener = TcpListener::bind(addr).await?;
    info!("校考星后端服务运行在 http://{}", listener.local_addr()?);

    axum::serve(listener, app).await?;
    Ok(())
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "xiaokaoxing_backend=debug,tower_http=info".into()),
        )
        .init();
}
