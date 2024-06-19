use anyhow::Result as AnyResult;
use ioc::{Bean, BeanFactory, Context};
use sqlx::SqlitePool;
use tracing::info;

#[derive(Bean)]
#[custom_factory]
pub(crate) struct Db;

impl BeanFactory for Db {
    type Bean = SqlitePool;
    fn build(ctx: &mut Context) -> ioc::Result<Self::Bean> {
        let database_url = ctx.get_config::<String>("db.url")?;

        info!("connecting to {database_url}:");
        let pool = SqlitePool::connect_lazy(database_url.as_str())
            .map_err(anyhow::Error::from)?;
        info!("build connection pool success!");

        Ok(pool)
    }
}

pub(crate) async fn init(_database_url: &str) -> AnyResult<()> {
    let db = Db::try_get()?;

    info!("migrating DB:");
    sqlx::migrate!("./migrations").run(db).await?;
    info!("migrate DB success!");
    Ok(())
}