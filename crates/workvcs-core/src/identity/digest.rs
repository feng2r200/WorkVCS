use crate::error::{Result, WorkVcsError};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Digest([u8; 32]);

impl Digest {
    pub const BYTE_LEN: usize = 32;
    pub const HEX_LEN: usize = 64;

    pub fn raw(bytes: &[u8]) -> Self {
        Self(*blake3::hash(bytes).as_bytes())
    }

    pub fn domain_separated(domain: &str, payload: &[u8]) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"workvcs-digest-v1\0");
        hasher.update(domain.as_bytes());
        hasher.update(b"\0");
        hasher.update(payload);
        Self(*hasher.finalize().as_bytes())
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_hex(self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(value: &str) -> Result<Self> {
        if value.len() != Self::HEX_LEN {
            return Err(WorkVcsError::DigestInvalid(
                "digest hex must be exactly 64 lowercase characters".to_owned(),
            ));
        }
        if !value
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_digit() || (*byte >= b'a' && *byte <= b'f'))
        {
            return Err(WorkVcsError::DigestInvalid(
                "digest hex must use lowercase hexadecimal".to_owned(),
            ));
        }
        let decoded = hex::decode(value).map_err(|error| {
            WorkVcsError::DigestInvalid(format!("digest hex decode failed: {error}"))
        })?;
        let bytes: [u8; 32] = decoded.try_into().map_err(|_| {
            WorkVcsError::DigestInvalid("digest hex decoded to wrong length".to_owned())
        })?;
        Ok(Self(bytes))
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl Serialize for Digest {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Digest {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::from_hex(&value).map_err(serde::de::Error::custom)
    }
}
