//! The template side of component contracts (`dpl.component/1`, LS1). The
//! document model — types, rules, the evolution check — is
//! `dpl_doc::component`; this module is the glue a template needs to BE its
//! contract: serde helpers for its fields, [`component_contract!`], and
//! [`instantiate`], which builds the gallery demo from the first example.

use dpl_doc::component::{example_values, ComponentContract};
use serde::de::DeserializeOwned;

use crate::registry::SlideTemplate;

/// `Tone` as authored: its hue id (`"amber"`), never an ordinal — the
/// stored value survives a reordered enum. `#[serde(with = "crate::contract::tone")]`.
pub mod tone {
    use dpl_coldlight::Tone;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(tone: &Tone, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(tone.hue_id())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Tone, D::Error> {
        let id = String::deserialize(deserializer)?;
        dpl_doc::component::tone_from_id(&id)
            .ok_or_else(|| serde::de::Error::custom(format!("no tone `{id}`")))
    }
}

/// `Option<Tone>` as authored: a hue id, or absent/null for "the slide's
/// tone". `#[serde(default, with = "crate::contract::tone_opt")]`.
pub mod tone_opt {
    use dpl_coldlight::Tone;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(tone: &Option<Tone>, serializer: S) -> Result<S::Ok, S::Error> {
        match tone {
            Some(tone) => serializer.serialize_some(tone.hue_id()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Tone>, D::Error> {
        match Option::<String>::deserialize(deserializer)? {
            None => Ok(None),
            Some(id) => dpl_doc::component::tone_from_id(&id)
                .map(Some)
                .ok_or_else(|| serde::de::Error::custom(format!("no tone `{id}`"))),
        }
    }
}

/// Declares a template's `contract()`: its `component.json`, parsed once.
/// Pass the text with `include_str!` written at the call site, so the path
/// is the template's own folder.
#[macro_export]
macro_rules! component_contract {
    ($text:expr) => {
        /// This template's contract (`dpl.component/1`), parsed once.
        pub fn contract() -> &'static dpl_doc::component::ComponentContract {
            static CONTRACT: std::sync::OnceLock<dpl_doc::component::ComponentContract> =
                std::sync::OnceLock::new();
            CONTRACT.get_or_init(|| {
                dpl_doc::component::parse($text)
                    .expect("component.json is a valid contract (the contract tests prove it)")
            })
        }
    };
}

/// A live template filled from its contract's example `index`, with its
/// interaction state reset (a fresh instance, as the gallery wants it).
pub fn instantiate<T>(contract: &ComponentContract, index: usize) -> Result<T, String>
where
    T: DeserializeOwned + SlideTemplate,
{
    let values = example_values(contract, index)
        .ok_or_else(|| format!("{} has no example {index}", contract.id))?;
    let mut template: T = serde_json::from_value(values)
        .map_err(|error| format!("{} example {index}: {error}", contract.id))?;
    template.restart();
    Ok(template)
}
