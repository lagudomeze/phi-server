use anyhow::Result as AnyResult;
use ioc::{Bean, BeanFactory, Context};
use sqlx::SqlitePool;
use tokio::runtime::Builder;
use tracing::info;

#[derive(Bean)]
#[custom_factory]
pub(crate) struct Db {}

impl BeanFactory for Db {
    type Bean = SqlitePool;
    fn build(ctx: &mut Context) -> ioc::Result<Self::Bean> {
        let database_url = ctx.get_config::<String>("db.url")?;

        let pool = Builder::new_current_thread()
            .enable_time()
            .build()?
            .block_on(init(database_url.as_str()))?;

        Ok(pool)
    }
}

async fn init(database_url: &str) -> AnyResult<SqlitePool> {
    info!("connecting to {database_url}:");
    let db = SqlitePool::connect(database_url).await?;
    info!("build connection pool success!");

    info!("migrating DB:");
    sqlx::migrate!("./migrations")
        .run(&db)
        .await?;
    info!("migrate DB success!");
    Ok(db)
}