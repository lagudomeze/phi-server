use uuid::Uuid;

pub mod biz;
pub mod mvc;
pub(crate) fn new_id() -> String {
    Uuid::new_v4().as_simple().to_string()
}