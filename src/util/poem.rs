use crate::common;
use http::uri::Scheme;
use poem::{FromRequest, Request, RequestBody};
use std::str::FromStr;
use tracing::log::warn;
use url::Url;

pub(crate) struct BaseUrl {
    base_url: String,
}

fn from(proto: &str) -> Option<Scheme> {
    match Scheme::from_str(proto) {
        Ok(scheme) => Some(scheme),
        Err(e) => {
            warn!("parse scheme {proto} failed: {e}");
            None
        }
    }
}

impl<'a> FromRequest<'a> for BaseUrl {
    async fn from_request(req: &'a Request, _: &mut RequestBody) -> poem::Result<Self> {
        let option = req.header("X-Forwarded-Proto").and_then(from);

        let scheme = option.as_ref().unwrap_or(req.scheme());

        let base_url = if let Some(host) = req.header("Host") {
            format!("{scheme}://{host}")
        } else {
            let host = req.local_addr();
            format!("{scheme}://{host}")
        };

        Ok(BaseUrl { base_url })
    }
}

impl BaseUrl {
    pub(crate) fn join(&self, path: &str) -> common::Result<Url> {
        Ok(Url::parse(&self.base_url)?.join(path)?)
    }
}
