use std::ops::Deref;
use ioc::BeanSpec;
use crate::auth::jwt::{Claims, JwtService};
use poem::Request;
use poem_openapi::{
    auth::ApiKey,
    SecurityScheme
};

#[derive(SecurityScheme)]
#[oai(
    ty = "api_key",
    key_name = "auth",
    key_in = "header",
    checker = "api_checker"
)]
pub(crate) struct JwtAuth(Claims);

impl Deref for JwtAuth {
    type Target = Claims;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Into<Claims> for JwtAuth {
    fn into(self) -> Claims {
        self.0
    }
}

async fn api_checker(_req: &Request, api_key: ApiKey) -> poem::Result<Claims> {
    let claims = JwtService::get().decode(&api_key.key)?;
    Ok(claims)
}