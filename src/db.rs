use std::sync::Arc;

use anyhow::Result as AnyResult;
use sqlx::SqlitePool;
use tracing::info;

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) db: SqlitePool,
}

pub(crate) async fn init(database_url: &str) -> AnyResult<AppState> {
    info!("connecting to {database_url}:");
    let pool = SqlitePool::connect(database_url).await?;
    info!("build connection pool success!");

    info!("migrating db:");
    sqlx::migrate!("db/migrations").run(&pool).await?;
    info!("migrate db success!");

    Ok(AppState {
        db: pool
    })
}