use std::{
    fs::{create_dir_all, exists, read, write},
    path::Path,
    path::PathBuf,
};

use ioc::{bean, BeanSpec, InitContext};
use jsonwebtoken::{Algorithm, decode, DecodingKey, encode, EncodingKey, get_current_timestamp, Header, Validation};
use ring::{
    rand::SystemRandom,
    signature::Ed25519KeyPair,
};
use serde::{Deserialize, Serialize};

use crate::common::Result;

struct Keys {
    encoding: EncodingKey,
    decoding: DecodingKey,
}

impl Keys {
    fn from(secret: &[u8]) -> Self {
        Self {
            encoding: EncodingKey::from_ed_der(secret),
            decoding: DecodingKey::from_ed_der(secret),
        }
    }

    fn new(document_path: impl AsRef<Path>) -> ioc::Result<Self> {
        let keys = if exists(&document_path)? {
            let bytes = read(&document_path)?;
            Self::from(&bytes)
        } else {
            let document = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new())
                .map_err(|err| {
                    ioc::IocError::Other(err.into())
                })?;
            if let Some(parent) = document_path.as_ref().parent() {
                create_dir_all(parent)?;
            }
            write(&document_path, document.as_ref())?;
            Self::from(document.as_ref())
        };
        Ok(keys)
    }
}

pub(crate) struct JwtService {
    keys: Keys,
    expire_secs: u64,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    name: String,
    id: String,
    exp: u64,
}

#[bean]
impl BeanSpec for JwtService {
    type Bean = Self;

    fn build(ctx: &mut impl InitContext) -> ioc::Result<Self::Bean> {
        let document_path = ctx.get_config::<PathBuf>("jwt.document_path")?;

        let keys = Keys::new(document_path)?;

        let expire_secs = ctx.get_config::<u64>("jwt.expire_seconds")?;

        Ok(JwtService {
            keys,
            expire_secs,
        })
    }
}

impl JwtService {
    pub fn new_claims(&self, name: String, id: String) -> Claims {
        Claims {
            name,
            id,
            exp: get_current_timestamp() + self.expire_secs,
        }
    }

    pub(crate) fn encode(&self, claims: &Claims) -> Result<String> {
        encode(&Header::new(Algorithm::EdDSA), claims, &self.keys.encoding)
            .map_err(Into::into)
    }

    pub(crate) fn decode(&self, token: &str) -> Result<Claims> {
        let mut validation = Validation::new(Algorithm::EdDSA);
        validation.reject_tokens_expiring_in_less_than = self.expire_secs;
        let result = decode::<Claims>(token, &self.keys.decoding, &validation)?;
        Ok(result.claims)
    }
}