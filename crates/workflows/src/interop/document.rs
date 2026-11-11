//! Strict document decoding: duplicate keys must not erase source behavior.

use crate::{refusal, MappingIssue, MappingResult};
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Number, Value};
use std::fmt;

pub const MAX_DOCUMENT_BYTES: usize = 1_048_576;

/// Parse bounded JSON without silently taking the last duplicate object key.
/// The normal serde_json recursion limit also remains enabled.
pub fn parse_document(text: &str) -> MappingResult<Value> {
    if text.len() > MAX_DOCUMENT_BYTES {
        return refusal("", "document exceeds the 1 MiB mapping limit");
    }
    serde_json::from_str::<UniqueValue>(text)
        .map(|value| value.0)
        .map_err(|error| {
            vec![MappingIssue {
                path: "".into(),
                reason: format!("invalid or ambiguous JSON: {error}"),
            }]
        })
}

struct UniqueValue(Value);

impl<'de> Deserialize<'de> for UniqueValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct UniqueVisitor;
        impl<'de> Visitor<'de> for UniqueVisitor {
            type Value = UniqueValue;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("JSON with unique object keys")
            }

            fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Bool(value)))
            }

            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Number(value.into())))
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Number(value.into())))
            }

            fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
                Number::from_f64(value)
                    .map(|number| UniqueValue(Value::Number(number)))
                    .ok_or_else(|| E::custom("non-finite number"))
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::String(value.into())))
            }

            fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::String(value)))
            }

            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Null))
            }

            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(value) = sequence.next_element::<UniqueValue>()? {
                    values.push(value.0);
                }
                Ok(UniqueValue(Value::Array(values)))
            }

            fn visit_map<A: MapAccess<'de>>(self, mut object: A) -> Result<Self::Value, A::Error> {
                let mut values = Map::new();
                while let Some(key) = object.next_key::<String>()? {
                    if values.contains_key(&key) {
                        return Err(de::Error::custom(format!("duplicate object key {key:?}")));
                    }
                    values.insert(key, object.next_value::<UniqueValue>()?.0);
                }
                Ok(UniqueValue(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(UniqueVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_behavior_is_rejected_at_every_depth() {
        for text in [
            r#"{"do": [], "do": []}"#,
            r#"{"do":[{"save":{"call":"one","call":"two"}}]}"#,
            r#"{"metadata":{"required":[],"required":["hidden"]}}"#,
        ] {
            assert!(parse_document(text).unwrap_err()[0]
                .reason
                .contains("duplicate"));
        }
    }

    #[test]
    fn decoding_preserves_values_and_refuses_trailing_or_excessive_input() {
        let text = r#"{"a":[null,true,false,0,-4,1.25,18446744073709551615,"text"]}"#;
        assert_eq!(
            parse_document(text).unwrap(),
            serde_json::from_str::<Value>(text).unwrap()
        );
        assert!(parse_document("{} {}").is_err());
        assert!(parse_document(&" ".repeat(MAX_DOCUMENT_BYTES + 1)).is_err());
        assert!(parse_document(&format!("{}0{}", "[".repeat(150), "]".repeat(150))).is_err());
    }
}
