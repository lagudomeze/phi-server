use uuid::Uuid;

pub mod biz;
pub mod mvc;
pub mod deploy;

pub(crate) fn new_id() -> String {
    Uuid::new_v4().as_simple().to_string()
}