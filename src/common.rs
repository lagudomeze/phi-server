use axum::http::StatusCode;
use axum::Json;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

pub(crate) mod approx_instant {
    use std::time::{Instant, SystemTime};

    use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(instant: &Instant, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        let system_now = SystemTime::now();
        let instant_now = Instant::now();
        let approx = system_now - (instant_now - *instant);
        approx.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Instant, D::Error>
        where
            D: Deserializer<'de>,
    {
        let de = SystemTime::deserialize(deserializer)?;
        let system_now = SystemTime::now();
        let instant_now = Instant::now();
        let duration = system_now.duration_since(de).map_err(Error::custom)?;
        let approx = instant_now - duration;
        Ok(approx)
    }
}

#[derive(Debug, ToSchema, Serialize, Default)]
pub(crate) struct Array<T>(Vec<T>);

impl<T> AsRef<Vec<T>> for Array<T> {
    fn as_ref(&self) -> &Vec<T> {
        &self.0
    }
}

impl<T> AsMut<Vec<T>> for Array<T> {
    fn as_mut(&mut self) -> &mut Vec<T> {
        &mut self.0
    }
}

impl<T> From<Vec<T>> for Array<T> {
    fn from(value: Vec<T>) -> Self {
        Self(value)
    }
}

impl<T> IntoResponse for Array<T> where T: Serialize, {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self.0)).into_response()
    }
}