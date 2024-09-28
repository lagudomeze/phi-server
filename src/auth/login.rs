use crate::auth::apikey::JwtAuth;
use crate::auth::jwt::Claims;
use crate::auth::user::NewUser;
use crate::{
    auth::{jwt::JwtService, user::UserService},
    client::HttpClient,
    common::{self, LocationContext, PhiTags},
};
use cfg_rs::impl_enum;
use common::AppError::InvalidUsernameOrPassword;
use http::uri::Scheme;
use ioc::{mvc, Bean};
use poem::Request;
use poem_openapi::payload::Json;
use poem_openapi::{param::Query, Object, OpenApi};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, panic::Location, str::FromStr};
use chrono::{DateTime, Local};
use tracing::{debug, info};
use tracing::log::warn;

enum RedirectPolicy {
    Safe,
    Auto,
    Manual,
}

impl_enum!(RedirectPolicy {
    "safe" => RedirectPolicy::Safe
    "auto" => RedirectPolicy::Auto
    "manual" => RedirectPolicy::Manual
});

#[derive(Bean)]
pub struct LoginMvc {
    #[inject(bean)]
    oauth: &'static Oauth2,
    #[inject(bean)]
    jwt: &'static JwtService,
}

#[derive(Bean)]
pub struct Oauth2 {
    #[inject(config = "oauth.authorization-url")]
    authorization_url: String,
    #[inject(config = "oauth.client-id")]
    client_id: String,
    #[inject(config = "oauth.client-secret")]
    client_secret: String,
    #[inject(config = "oauth.scopes")]
    scopes: Vec<String>,
    #[inject(bean)]
    service: &'static UserService,
    #[inject(bean)]
    jwt: &'static JwtService,
    #[inject(bean)]
    client: &'static HttpClient,
    #[inject(config = "oauth.redirect-policy")]
    redirect_policy: RedirectPolicy,
    #[inject(config = "oauth.redirect-url")]
    redirect_url: String,
    #[inject(config = "admin.name")]
    admin_name: String,
    #[inject(config = "admin.pass")]
    admin_pass: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct AccessTokenResult {
    access_token: String,
    scope: String,
    token_type: String,
}

pub trait AuthedUser {
    fn user_id(&self) -> Cow<'_, str>;

    fn name(&self) -> Cow<'_, str>;
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
    fn user_id(&self) -> Cow<'_, str> {
        Cow::Owned(format!("gh_{}", self.id))
    }

    fn name(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.name)
    }
}

impl Oauth2 {
    fn redirect_uri(&self, req: &Request) -> Option<Cow<'_, str>> {
        match self.redirect_policy {
            RedirectPolicy::Safe => None,
            RedirectPolicy::Auto => {
                let scheme = req
                    .header("X-Forwarded-Proto")
                    .and_then(|proto| {
                        Scheme::from_str(proto)
                            .map(Cow::Owned)
                            .inspect_err(|e| {
                                warn!("parse scheme {proto} failed: {e}");
                            })
                            .ok()
                    })
                    .unwrap_or(Cow::Borrowed(req.scheme()));

                let host = req
                    .header("Host")
                    .map(Cow::Borrowed)
                    .unwrap_or_else(|| Cow::Owned(req.local_addr().to_string()));

                Some(Cow::Owned(format!(
                    "{scheme}://{host}/api/auth/oauth2_login"
                )))
            }
            RedirectPolicy::Manual => Some(Cow::Borrowed(&self.redirect_url)),
        }
    }

    pub fn login_url(&self, req: &Request) -> common::Result<Url> {
        let mut url = Url::parse(&self.authorization_url)?;

        url.query_pairs_mut()
            .append_pair("client_id", &self.client_id)
            .append_pair("scope", &self.scopes.join(" "))
            .append_pair("response_type", "code");

        if let Some(redirect_uri) = self.redirect_uri(req) {
            url.query_pairs_mut()
                .append_pair("redirect_uri", &redirect_uri);
        }

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

        let user = client
            .get("https://api.github.com/user")
            .bearer_auth(result.access_token)
            .send()
            .await
            .location("get user info", Location::caller())?
            .json::<AuthedGithubUser>()
            .await
            .location("get access_token parse json failed", Location::caller())?;

        let user_id = user.user_id();
        let name = user.name();
        if !self.service.exists_by_id(user_id.as_ref()).await? {
            let new_user = NewUser::new(user_id.as_ref(), name.as_ref(), &user.email);
            self.service.create_user(new_user).await?;
            info!("user:{} id:{} created", name, user_id);
        }

        let claims = self.jwt.new_claims(name.into(), user_id.into());

        let token = self.jwt.encode(&claims)?;

        Ok(token)
    }

    pub async fn admin_login(&self, name: impl AsRef<str>, pass: impl AsRef<str>) -> common::Result<String> {
        if self.admin_name == name.as_ref() && self.admin_pass == pass.as_ref() {
            let user_id = "phi_super_admin";
            let name = &self.admin_name;
            if !self.service.exists_by_id(user_id).await? {
                let new_user = NewUser::new(user_id, name, "buildin");
                self.service.create_user(new_user).await?;
                info!("user:{} id:{} created", name, user_id);
            }

            let claims = self.jwt.new_claims(name.into(), user_id.into());

            let token = self.jwt.encode(&claims)?;
            Ok(token)
        } else {
            Err(InvalidUsernameOrPassword)
        }
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

#[derive(Debug, Deserialize, Serialize, Object)]
pub(crate) struct LoginRequest {
    username: String,
    password: String,
}

#[mvc]
#[OpenApi(prefix_path = "/api/auth", tag = PhiTags::Auth)]
impl LoginMvc {
    #[oai(path = "/oauth2_login_url/github", method = "get")]
    async fn oauth2_login_url(&self, req: &Request) -> common::Result<common::Response<LoginUrl>> {
        let url = self.oauth.login_url(req)?.to_string();
        Ok(common::Response::ok(LoginUrl { url }))
    }

    #[oai(path = "/oauth2_login", method = "get")]
    async fn login_by_github_code(
        &self,
        code: Query<String>,
    ) -> common::Result<common::Response<LoginResult>> {
        let token = self
            .oauth
            .login_by_github_code(&code.0)
            .await
            .location("login failed", Location::caller())?;
        Ok(common::Response::ok(LoginResult { token }))
    }

    #[oai(path = "/admin_login", method = "post")]
    async fn admin_login(
        &self,
        request: Json<LoginRequest>,
    ) -> common::Result<common::Response<LoginResult>> {
        let token = self
            .oauth
            .admin_login(&request.username, &request.password)
            .await
            .location("login failed", Location::caller())?;
        Ok(common::Response::ok(LoginResult { token }))
    }

    #[oai(path = "/token_refresh", method = "post")]
    async fn token_refresh(&self, auth: JwtAuth) -> common::Result<common::Response<LoginResult>> {
        let Claims { name, id, .. } = auth.into_inner();

        let claims = self.jwt.new_claims(name, id);

        let token = self
            .jwt.encode(&claims)
            .location("login failed", Location::caller())?;

        debug!("token {token} will expire at:{:#?}", DateTime::from_timestamp(claims.exp as i64, 0).map(|time| time.with_timezone(&Local)));

        Ok(common::Response::ok(LoginResult { token }))
    }
}
