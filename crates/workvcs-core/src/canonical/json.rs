use crate::canonical::CanonicalValue;
use crate::error::{Result, WorkVcsError};
use serde::Deserialize;
use std::cmp::Ordering;

pub fn parse_canonical_json(input: &[u8]) -> Result<CanonicalValue> {
    let mut deserializer = serde_json::Deserializer::from_slice(input);
    let value = CanonicalValue::deserialize(&mut deserializer).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!("invalid canonical JSON: {error}"))
    })?;
    deserializer.end().map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!("trailing JSON content: {error}"))
    })?;
    Ok(value)
}

pub fn canonical_bytes(value: &CanonicalValue) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    encode_value(value, &mut output)?;
    Ok(output)
}

fn encode_value(value: &CanonicalValue, output: &mut Vec<u8>) -> Result<()> {
    match value {
        CanonicalValue::Null => output.extend_from_slice(b"null"),
        CanonicalValue::Bool(false) => output.extend_from_slice(b"false"),
        CanonicalValue::Bool(true) => output.extend_from_slice(b"true"),
        CanonicalValue::Integer(integer) => {
            output.extend_from_slice(integer.get().to_string().as_bytes());
        }
        CanonicalValue::String(string) => {
            let encoded = serde_json::to_string(string).map_err(|error| {
                WorkVcsError::CanonicalEncodingInvalid(format!("string encode failed: {error}"))
            })?;
            output.extend_from_slice(encoded.as_bytes());
        }
        CanonicalValue::Array(values) => {
            output.push(b'[');
            for (index, item) in values.iter().enumerate() {
                if index > 0 {
                    output.push(b',');
                }
                encode_value(item, output)?;
            }
            output.push(b']');
        }
        CanonicalValue::Object(entries) => {
            let mut entries = entries.iter().collect::<Vec<_>>();
            entries.sort_by(|left, right| compare_utf16(&left.0, &right.0));

            output.push(b'{');
            for (index, (key, item)) in entries.iter().enumerate() {
                if index > 0 {
                    output.push(b',');
                }
                let encoded_key = serde_json::to_string(key).map_err(|error| {
                    WorkVcsError::CanonicalEncodingInvalid(format!("key encode failed: {error}"))
                })?;
                output.extend_from_slice(encoded_key.as_bytes());
                output.push(b':');
                encode_value(item, output)?;
            }
            output.push(b'}');
        }
    }
    Ok(())
}

fn compare_utf16(left: &str, right: &str) -> Ordering {
    left.encode_utf16().cmp(right.encode_utf16())
}

#[cfg(test)]
mod tests {
    use super::compare_utf16;
    use std::cmp::Ordering;

    #[test]
    fn utf16_order_differs_from_scalar_order_for_supplementary_codepoints() {
        assert_eq!(compare_utf16("\u{e000}", "\u{10000}"), Ordering::Greater);
    }
}
