use serde_json::Value;

pub(super) fn contains_secret_bearing_field(value: &Value) -> bool {
    match value {
        Value::Object(entries) => entries
            .iter()
            .any(|(key, value)| is_secret_bearing_key(key) || contains_secret_bearing_field(value)),
        Value::Array(values) => values.iter().any(contains_secret_bearing_field),
        _ => false,
    }
}

fn is_secret_bearing_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase().replace('-', "_");
    let compact = normalized.replace('_', "");
    matches!(
        compact.as_str(),
        "password"
            | "passwd"
            | "secret"
            | "token"
            | "accesstoken"
            | "refreshtoken"
            | "apikey"
            | "authorization"
            | "cookie"
            | "privatekey"
            | "clientsecret"
            | "credential"
            | "credentials"
    )
}
