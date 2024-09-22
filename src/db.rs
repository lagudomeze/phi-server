use std::str::FromStr;

use anyhow::Result as AnyResult;
use ioc::{bean, BeanSpec, InitContext};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode::Wal, SqlitePoolOptions},
    SqlitePool,
};
use tokio::runtime::Builder;
use tracing::info;

pub(crate) struct Db {}

#[bean]
impl BeanSpec for Db {
    type Bean = SqlitePool;
    fn build(ctx: &mut impl InitContext) -> ioc::Result<Self::Bean> {
        let database_url = ctx.get_config::<String>("db.url")?;
        let max_connections = ctx.get_config::<u32>("db.max-connections")?;

        let pool = Builder::new_current_thread()
            .enable_time()
            .build()?
            .block_on(init(database_url.as_str(), max_connections))?;

        Ok(pool)
    }
}

async fn init(database_url: &str, max_connections: u32) -> AnyResult<SqlitePool> {
    info!("connecting to {database_url}:");
    let options = SqliteConnectOptions::from_str(database_url)?.journal_mode(Wal);

    let db = SqlitePoolOptions::new()
        .max_connections(max_connections)
        .connect_with(options)
        .await?;
    info!("build connection pool success!");

    info!("migrating DB:");
    sqlx::migrate!("./migrations").run(&db).await?;
    info!("migrate DB success!");
    Ok(db)
}
