use crate::common::LocationContext;
use http::HeaderValue;
use ioc::{bean, BeanSpec, InitContext};
use reqwest::header::{HeaderMap, ACCEPT};
use std::panic::Location;

pub(crate) struct HttpClient {
    client: reqwest::Client,
}

impl HttpClient {
    pub(crate) fn rest(&self) -> &reqwest::Client {
        &self.client
    }
}

#[bean]
impl BeanSpec for HttpClient {
    type Bean = HttpClient;

    fn build(_: &mut impl InitContext) -> ioc::Result<Self::Bean> {
        let mut headers = HeaderMap::new();
        headers.insert(
            reqwest::header::USER_AGENT,
            HeaderValue::from_static("phi_server"),
        );
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .location("build http client", Location::caller())?;

        Ok(Self::Bean { client })
    }
}
