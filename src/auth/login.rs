use ioc::{Bean, mvc};

use crate::common;

use super::user::UserRepo;

#[derive(Bean)]
pub struct LoginMvc {
    #[inject]
    repo: &'static UserRepo,
}

#[mvc]
impl LoginMvc {
    #[oai(path = "/login", method = "post")]
    async fn login(&self) -> common::Result<common::Response<String>> {
        Ok(common::Response::ok("login".to_string()))
    }
}
