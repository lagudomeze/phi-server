use chrono::NaiveDateTime;
use ioc::{Bean, mvc};
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::common::Response;
use crate::db::Db;

#[derive(Bean)]
pub struct UserRepo {
    #[inject(bean = Db)]
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

impl User {
    pub fn new(id: String, name: String, source: String) -> Self {
        Self {
            id,
            name,
            source: Some(source),
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}

#[derive(Bean)]
pub struct UserService {
    #[inject(bean = Db)]
    db: &'static SqlitePool,
}

impl UserService {
    pub(crate) async fn exists_by_id(&self, id: &str) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar!("SELECT COUNT(1) FROM users WHERE id = ?1", id)
            .fetch_one(self.db)
            .await
            .map(|count| count > 0)
    }

    pub(crate) async fn create_user(&self, id: &str, name: &str, source: String) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now();
        let native_utc = now.naive_utc();
        sqlx::query!("INSERT INTO users (id, name, source, created_at) VALUES (?1, ?2, ?3, ?4)", id, name, source, native_utc)
            .execute(self.db)
            .await
            .map(|_| ())
    }
}