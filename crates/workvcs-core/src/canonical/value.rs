use crate::error::{Result, WorkVcsError};
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::collections::HashSet;
use std::fmt;

pub const MIN_SAFE_INTEGER: i64 = -9_007_199_254_740_991;
pub const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SafeInteger(i64);

impl SafeInteger {
    pub fn new(value: i64) -> Result<Self> {
        if (MIN_SAFE_INTEGER..=MAX_SAFE_INTEGER).contains(&value) {
            Ok(Self(value))
        } else {
            Err(WorkVcsError::CanonicalEncodingInvalid(format!(
                "integer {value} is outside the JSON safe integer range"
            )))
        }
    }

    pub fn get(self) -> i64 {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CanonicalValue {
    Null,
    Bool(bool),
    Integer(SafeInteger),
    String(String),
    Array(Vec<CanonicalValue>),
    Object(Vec<(String, CanonicalValue)>),
}

impl CanonicalValue {
    pub fn safe_integer(value: i64) -> Result<Self> {
        Ok(Self::Integer(SafeInteger::new(value)?))
    }

    pub fn object(entries: Vec<(String, CanonicalValue)>) -> Result<Self> {
        reject_duplicate_keys(&entries)?;
        Ok(Self::Object(entries))
    }
}

impl<'de> Deserialize<'de> for CanonicalValue {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(CanonicalValueVisitor)
    }
}

struct CanonicalValueVisitor;

impl<'de> Visitor<'de> for CanonicalValueVisitor {
    type Value = CanonicalValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a WorkVCS canonical semantic JSON value")
    }

    fn visit_unit<E>(self) -> std::result::Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(CanonicalValue::Null)
    }

    fn visit_none<E>(self) -> std::result::Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(CanonicalValue::Null)
    }

    fn visit_bool<E>(self, value: bool) -> std::result::Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(CanonicalValue::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> std::result::Result<Self::Value, E>
    where
        E: de::Error,
    {
        SafeInteger::new(value)
            .map(CanonicalValue::Integer)
            .map_err(E::custom)
    }

    fn visit_u64<E>(self, value: u64) -> std::result::Result<Self::Value, E>
    where
        E: de::Error,
    {
        let signed = i64::try_from(value)
            .map_err(|_| E::custom("integer is outside the JSON safe integer range"))?;
        self.visit_i64(signed)
    }

    fn visit_f64<E>(self, _value: f64) -> std::result::Result<Self::Value, E>
    where
        E: de::Error,
    {
        Err(E::custom(
            "floating point numbers are not valid WorkVCS canonical semantic JSON",
        ))
    }

    fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(CanonicalValue::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> std::result::Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(CanonicalValue::String(value))
    }

    fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = seq.next_element()? {
            values.push(value);
        }
        Ok(CanonicalValue::Array(values))
    }

    fn visit_map<A>(self, mut map: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut seen = HashSet::new();
        let mut entries = Vec::new();
        while let Some(key) = map.next_key::<String>()? {
            if !seen.insert(key.clone()) {
                return Err(de::Error::custom(format!(
                    "duplicate object key {key:?} is invalid"
                )));
            }
            let value = map.next_value()?;
            entries.push((key, value));
        }
        Ok(CanonicalValue::Object(entries))
    }
}

fn reject_duplicate_keys(entries: &[(String, CanonicalValue)]) -> Result<()> {
    let mut seen = HashSet::new();
    for (key, _) in entries {
        if !seen.insert(key) {
            return Err(WorkVcsError::CanonicalEncodingInvalid(format!(
                "duplicate object key {key:?} is invalid"
            )));
        }
    }
    Ok(())
}
