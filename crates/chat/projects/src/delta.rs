//! Typed JSON differences keep autosaves proportional to edited content.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub(crate) enum Delta {
    Replace {
        value: Value,
    },
    Object {
        entries: BTreeMap<String, Option<Delta>>,
    },
    Array {
        length: usize,
        entries: Vec<(usize, Delta)>,
    },
    String {
        keep: usize,
        suffix: String,
    },
}

impl Delta {
    pub(crate) fn between(before: &Value, after: &Value) -> Self {
        match (before, after) {
            (Value::Object(before), Value::Object(after)) => {
                let mut entries = BTreeMap::new();
                for key in before.keys().filter(|key| !after.contains_key(*key)) {
                    entries.insert(key.clone(), None);
                }
                for (key, value) in after {
                    if before.get(key) != Some(value) {
                        let change = before
                            .get(key)
                            .map(|old| Self::between(old, value))
                            .unwrap_or_else(|| Self::Replace {
                                value: value.clone(),
                            });
                        entries.insert(key.clone(), Some(change));
                    }
                }
                Self::Object { entries }
            }
            (Value::Array(before), Value::Array(after)) => {
                let entries = after
                    .iter()
                    .enumerate()
                    .filter_map(|(index, value)| {
                        if before.get(index) == Some(value) {
                            return None;
                        }
                        let change = before
                            .get(index)
                            .map(|old| Self::between(old, value))
                            .unwrap_or_else(|| Self::Replace {
                                value: value.clone(),
                            });
                        Some((index, change))
                    })
                    .collect();
                Self::Array {
                    length: after.len(),
                    entries,
                }
            }
            (Value::String(before), Value::String(after)) => {
                let mut keep = before
                    .bytes()
                    .zip(after.bytes())
                    .take_while(|(a, b)| a == b)
                    .count();
                while !before.is_char_boundary(keep) || !after.is_char_boundary(keep) {
                    keep -= 1;
                }
                Self::String {
                    keep,
                    suffix: after[keep..].to_owned(),
                }
            }
            _ => Self::Replace {
                value: after.clone(),
            },
        }
    }

    pub(crate) fn apply(self, target: &mut Value) -> Result<(), String> {
        match self {
            Self::Replace { value } => *target = value,
            Self::Object { entries } => {
                let object = target
                    .as_object_mut()
                    .ok_or("Object delta has no object base")?;
                for (key, change) in entries {
                    match change {
                        None => {
                            object
                                .remove(&key)
                                .ok_or("Object delta removes an absent key")?;
                        }
                        Some(Self::Replace { value }) => {
                            object.insert(key, value);
                        }
                        Some(change) => change.apply(
                            object
                                .get_mut(&key)
                                .ok_or("Object delta has an absent key")?,
                        )?,
                    }
                }
            }
            Self::Array { length, entries } => {
                let array = target
                    .as_array_mut()
                    .ok_or("Array delta has no array base")?;
                array.truncate(length);
                let mut previous = None;
                for (index, change) in entries {
                    if previous.is_some_and(|previous| previous >= index) {
                        return Err("Array delta indices must be unique and increasing".into());
                    }
                    previous = Some(index);
                    if index >= length {
                        return Err("Array delta index exceeds its final length".into());
                    }
                    if let Some(value) = array.get_mut(index) {
                        change.apply(value)?;
                    } else if index == array.len() {
                        let Self::Replace { value } = change else {
                            return Err("Appended array delta needs a value".into());
                        };
                        array.push(value);
                    } else {
                        return Err("Array delta skips an index".into());
                    }
                }
                if array.len() != length {
                    return Err("Array delta has an incomplete tail".into());
                }
            }
            Self::String { keep, suffix } => {
                let Value::String(value) = target else {
                    return Err("String delta has no string base".into());
                };
                if !value.is_char_boundary(keep) {
                    return Err("String delta prefix is out of bounds".into());
                }
                value.truncate(keep);
                value.push_str(&suffix);
            }
        }
        Ok(())
    }
}
