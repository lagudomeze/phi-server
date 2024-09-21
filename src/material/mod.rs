use poem_openapi::Enum;
use serde::{Deserialize, Serialize};

pub mod biz;
pub mod mvc;
pub mod storage;

pub const TYPE_VIDEO: u16 = 1;
pub const TYPE_IMAGE: u16 = 2;

#[derive(Serialize, Deserialize, Debug, Enum)]
pub enum MaterialType {
    Video,
    Image,
}

impl MaterialType {
    pub(crate) fn value(&self) -> u16 {
        match self {
            MaterialType::Video => TYPE_VIDEO,
            MaterialType::Image => TYPE_IMAGE,
        }
    }
}

pub const STATE_OK: u16 = 0;
