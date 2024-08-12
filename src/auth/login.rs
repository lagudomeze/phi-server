use cfg_rs::impl_enum;
use http::uri::Scheme;
use ioc::{mvc, Bean};
use poem::Request;
use poem_openapi::{
    param::Query,
    Object,
    OpenApi
};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    panic::Location,
    str::FromStr
};
use tracing::info;
use tracing::log::warn;
use crate::{
    auth::{
        jwt::JwtService,
        user::UserService,
    },
    client::HttpClient,
    common::{
        self,
        LocationContext,
        PhiTags
    },
};

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
    #[inject(config = "oauth.redirect_policy")]
    redirect_policy: RedirectPolicy,
    #[inject(config = "oauth.redirect_url")]
    redirect_url: String
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
    fn redirect_uri(&self, req: &Request) -> Option<Cow<'_, str>> {
        match self.redirect_policy {
            RedirectPolicy::Safe => None,
            RedirectPolicy::Auto => {
                let scheme = req.header("X-Forwarded-Proto")
                    .and_then(|proto| {
                        match Scheme::from_str(proto) {
                            Ok(scheme) => {
                                Some(scheme)
                            }
                            Err(e) => {
                                warn!("parse scheme {proto} failed: {e}");
                                None
                            }
                        }
                    })
                    .unwrap_or(req.scheme().clone());

                let host = req.header("Host")
                    .map(Cow::Borrowed)
                    .unwrap_or_else(|| Cow::Owned(req.local_addr().to_string()));

                Some(Cow::Owned(format!("{scheme}://{host}/api/auth/oauth2_login")))
            }
            RedirectPolicy::Manual => Some(Cow::Borrowed(&self.redirect_url)),
        }
    }
}

impl Oauth2 {
    pub fn login_url(&self, req: &Request) -> common::Result<Url> {
        let mut url = Url::parse(&self.authorization_url)?;

        {
            let mut binding = url.query_pairs_mut();
            binding
                .append_pair("client_id", &self.client_id)
                .append_pair("scope", &self.scopes.join(" "))
                .append_pair("response_type", "code");

            if let Some(redirect_uri) = self.redirect_uri(req) {
                binding.append_pair("redirect_uri", &*redirect_uri);
            }
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

        let user = client.get("https://api.github.com/user")
            .bearer_auth(result.access_token)
            .send()
            .await
            .location("get user info", Location::caller())?
            .json::<AuthedGithubUser>()
            .await
            .location("get access_token parse json failed", Location::caller())?;

        let user_id = user.user_id();
        let name = user.name();
        if !self.service.exists_by_id(&user_id).await? {
            self.service.create_user(&user_id, &name, user.email).await?;
            info!("user:{} id:{} created", name, user_id);
        }

        let claims = self.jwt.new_claims(name, user_id);

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
    async fn oauth2_login_url(&self, req: &Request) -> common::Result<common::Response<LoginUrl>> {
        let url = self.oauth.login_url(req)?.to_string();
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
