use std::sync::{OnceLock};

use anyhow::Result as AnyResult;
use sqlx::SqlitePool;
use tracing::info;

static DB: OnceLock<SqlitePool> = OnceLock::new();

pub(crate) fn db() -> &'static SqlitePool {
    DB.get().expect("not connect!")
}

pub(crate) async fn init(database_url: &str) -> AnyResult<()> {
    let db = DB.get_or_init(|| {
        info!("connecting to {database_url}:");
        let pool = SqlitePool::connect_lazy(database_url).expect("connect failed");
        info!("build connection pool success!");
        pool
    });

    info!("migrating DB:");
    sqlx::migrate!("DB/migrations").run(db).await?;
    info!("migrate DB success!");
    Ok(())
}