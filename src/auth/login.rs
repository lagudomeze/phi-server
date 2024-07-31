use ioc::{mvc, Bean};

use crate::{auth::user::UserRepo, common};

#[derive(Bean)]
pub struct LoginMvc {
    #[inject(bean)]
    repo: &'static UserRepo,
}

#[mvc]
impl LoginMvc {
    #[oai(path = "/login", method = "post")]
    async fn login(&self) -> common::Result<common::Response<String>> {
        Ok(common::Response::ok("login".to_string()))
    }
}
