use std::panic::Location;
use ioc::{Bean, mvc};
use poem_openapi::{
    Object, param::Query,
};
use poem_openapi::OpenApi;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{
    auth::{
        jwt::JwtService,
        user::{
            UserService,
        },
    },
    common::{
        self,
        PhiTags,
    },
};
use crate::client::HttpClient;
use crate::common::LocationContext;

#[derive(Bean)]
pub struct LoginMvc {
    #[inject(bean)]
    oauth: &'static Oauth2,
}

#[derive(Bean)]
pub struct Oauth2 {
    #[inject(config = "oauth.authorization_url")]
    authorization_url: String,
    #[inject(config = "oauth.client_id")]
    client_id: String,
    #[inject(config = "oauth.client_secret")]
    client_secret: String,
    #[inject(config = "oauth.scopes")]
    scopes: Vec<String>,
    #[inject(bean)]
    service: &'static UserService,
    #[inject(bean)]
    jwt: &'static JwtService,
    #[inject(bean)]
    client: &'static HttpClient,
}

#[derive(Serialize, Deserialize, Debug)]
struct AccessTokenResult {
    access_token: String,
    scope: String,
    token_type: String,
}

pub trait AuthedUser {
    fn user_id(&self) -> String;

    fn name(&self) -> String;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthedGithubUser {
    id: i64,
    login: String,
    name: String,
    email: String,
    avatar_url: String,
}

impl AuthedUser for AuthedGithubUser {
    fn user_id(&self) -> String {
        format!("gh_{}", self.id)
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

impl Oauth2 {
    pub fn login_url(&self) -> common::Result<Url> {
        let mut url = Url::parse(&self.authorization_url)?;
        url.query_pairs_mut()
            .append_pair("client_id", &self.client_id)
            .append_pair("scope", &self.scopes.join(" "))
            .append_pair("response_type", "code");
        Ok(url)
    }

    pub async fn login_by_github_code(&self, code: impl AsRef<str>) -> common::Result<String> {
        let client = self.client.rest();

        let mut url = Url::parse("https://github.com/login/oauth/access_token")?;

        url.query_pairs_mut()
            .append_pair("grant_type", "authorization_code")
            .append_pair("client_id", &self.client_id)
            .append_pair("client_secret", &self.client_secret)
            .append_pair("code", code.as_ref());

        let result = client
            .post(url)
            .send()
            .await
            .location("get access_token", Location::caller())?
            .json::<AccessTokenResult>()
            .await
            .location("get access_token parse json failed", Location::caller())?;

        let user = client.get("https://api.github.com/user")
            .bearer_auth(result.access_token)
            .send()
            .await
            .location("get user info", Location::caller())?
            .json::<AuthedGithubUser>()
            .await
            .location("get access_token parse json failed", Location::caller())?;

        let user_id = user.user_id();
        if !self.service.exists_by_id(&user_id).await? {
            self.service.create_user(&user_id, &user.name, user.email).await?;
            info!("user:{} id:{} created", user.name, user_id);
        }

        let claims = self.jwt.new_claims(user.name, user_id);

        let token = self.jwt.encode(&claims)?;

        Ok(token)
    }
}

#[derive(Serialize, Deserialize, Debug, Object)]
pub(crate) struct LoginUrl {
    url: String,
}

#[derive(Serialize, Deserialize, Debug, Object)]
pub(crate) struct LoginResult {
    token: String,
}

#[mvc]
#[OpenApi(prefix_path = "/api/auth", tag = PhiTags::Auth)]
impl LoginMvc {
    #[oai(path = "/oauth2_login_url/github", method = "get")]
    async fn oauth2_login_url(&self) -> common::Result<common::Response<LoginUrl>> {
        let url = self.oauth.login_url()?.to_string();
        Ok(common::Response::ok(LoginUrl { url }))
    }

    #[oai(path = "/oauth2_login", method = "get")]
    async fn login_by_github_code(&self, code: Query<String>) -> common::Result<common::Response<LoginResult>> {
        let token = self
            .oauth
            .login_by_github_code(&code.0)
            .await
            .location("login failed", Location::caller())?;
        Ok(common::Response::ok(LoginResult { token }))
    }
}
