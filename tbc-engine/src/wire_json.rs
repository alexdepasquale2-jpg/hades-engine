//! JSON helpers for u128 IDs on the wire (ULIDs exceed `serde_json::Number`).

use serde::{Deserialize, Deserializer, Serializer};

pub fn serialize_u128_as_str<S>(value: &u128, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_string())
}

pub fn deserialize_u128_from_str_or_num<'de, D>(deserializer: D) -> Result<u128, D::Error>
where
    D: Deserializer<'de>,
{
    let v = serde_json::Value::deserialize(deserializer)?;
    match v {
        serde_json::Value::String(s) => s.parse().map_err(serde::de::Error::custom),
        serde_json::Value::Number(n) => n
            .as_u64()
            .map(|x| x as u128)
            .ok_or_else(|| serde::de::Error::custom("u128 number out of range")),
        _ => Err(serde::de::Error::custom("expected string or number for u128")),
    }
}

pub mod compat {
    use super::{deserialize_u128_from_str_or_num, serialize_u128_as_str};
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(value: &u128, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_u128_as_str(value, serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u128, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_u128_from_str_or_num(deserializer)
    }
}

pub fn id_json(value: u128) -> serde_json::Value {
    serde_json::Value::String(value.to_string())
}
