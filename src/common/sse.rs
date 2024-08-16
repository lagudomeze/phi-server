use poem_openapi::Object;
use serde::{Deserialize, Serialize};

#[derive(Object, Serialize, Deserialize)]
pub(crate) struct FormatedEvent {
    pub(crate) id: String,
    pub(crate) progress: i16,
    pub(crate) state: String,
}
