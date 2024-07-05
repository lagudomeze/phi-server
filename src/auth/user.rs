use chrono::NaiveDateTime;
use ioc::{Bean, mvc};
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::common::Response;

#[derive(Bean)]
pub struct UserRepo {
    #[inject(Db)]
    db: &'static SqlitePool,
}

#[mvc]
impl UserRepo {
    #[oai(path = "/manager/users", method = "get")]
    async fn users(&self) -> crate::common::Result<Response<Vec<User>>> {
        let result: Vec<User> = sqlx::query_as("SELECT id, name, source, created_at FROM users")
            .fetch_all(self.db)
            .await
            .map_err(anyhow::Error::from)?;
        Ok(Response::ok(result))
    }
}

#[derive(sqlx::FromRow, Serialize, Deserialize, Debug, Object)]
pub(crate) struct User {
    id: String,
    name: String,
    source: Option<String>,
    created_at: NaiveDateTime,
}