//! Per-stage artifacts — tolerant TOML on disk beside the pack
//! (`packs/<app-id>/foundry/*.toml`). Same document law as the registry:
//! tmp → rename, never clobber in place.

use std::fs;
use std::path::Path;
use toml::{Table, Value};

/// One anchor row persisted from the minted fabric.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnchorNode {
    pub path: String,
    pub role: String,
    pub name: String,
    pub actions: Vec<String>,
}

/// The Fabric stage artifact — the anchor map plus the elected seed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anchors {
    pub atspi_name: String,
    pub minted_ms: u64,
    pub seed_control: Option<String>,
    pub nodes: Vec<AnchorNode>,
}

impl Anchors {
    pub fn to_table(&self) -> Table {
        let mut doc = Table::new();
        doc.insert("schema".into(), Value::Integer(1));
        doc.insert("atspi_name".into(), Value::String(self.atspi_name.clone()));
        doc.insert("minted_ms".into(), Value::Integer(self.minted_ms as i64));
        if let Some(seed) = &self.seed_control {
            doc.insert("seed_control".into(), Value::String(seed.clone()));
        }
        let rows = self
            .nodes
            .iter()
            .map(|n| {
                let mut t = Table::new();
                t.insert("path".into(), Value::String(n.path.clone()));
                t.insert("role".into(), Value::String(n.role.clone()));
                t.insert("name".into(), Value::String(n.name.clone()));
                if !n.actions.is_empty() {
                    t.insert(
                        "actions".into(),
                        Value::Array(n.actions.iter().cloned().map(Value::String).collect()),
                    );
                }
                Value::Table(t)
            })
            .collect();
        doc.insert("anchor".into(), Value::Array(rows));
        doc
    }

    pub fn from_table(doc: &Table) -> Option<Self> {
        let nodes = match doc.get("anchor") {
            Some(Value::Array(rows)) => rows
                .iter()
                .filter_map(|row| {
                    let t = row.as_table()?;
                    Some(AnchorNode {
                        path: t.get("path")?.as_str()?.to_string(),
                        role: str_or_empty(t, "role"),
                        name: str_or_empty(t, "name"),
                        actions: str_array(t, "actions"),
                    })
                })
                .collect(),
            _ => Vec::new(),
        };
        Some(Self {
            atspi_name: str_or_empty(doc, "atspi_name"),
            minted_ms: doc
                .get("minted_ms")
                .and_then(Value::as_integer)
                .unwrap_or(0) as u64,
            seed_control: doc
                .get("seed_control")
                .and_then(Value::as_str)
                .map(str::to_string),
            nodes,
        })
    }
}

/// One probed verb's measured stats (the Probe stage artifact rows).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeRow {
    pub verb: String,
    pub rung: String,
    pub oracle: String,
    pub runs: u32,
    pub effect_ok: u32,
    pub total_ms: u64,
}

impl ProbeRow {
    /// Effect rate — the pack's `reliability` value. Measured, never asserted.
    pub fn reliability(&self) -> f64 {
        if self.runs == 0 {
            return 0.0;
        }
        f64::from(self.effect_ok) / f64::from(self.runs)
    }
}

pub fn probes_to_table(rows: &[ProbeRow]) -> Table {
    let mut doc = Table::new();
    doc.insert("schema".into(), Value::Integer(1));
    let arr = rows
        .iter()
        .map(|r| {
            let mut t = Table::new();
            t.insert("verb".into(), Value::String(r.verb.clone()));
            t.insert("rung".into(), Value::String(r.rung.clone()));
            t.insert("oracle".into(), Value::String(r.oracle.clone()));
            t.insert("runs".into(), Value::Integer(i64::from(r.runs)));
            t.insert("effect_ok".into(), Value::Integer(i64::from(r.effect_ok)));
            t.insert("total_ms".into(), Value::Integer(r.total_ms as i64));
            Value::Table(t)
        })
        .collect();
    doc.insert("probe".into(), Value::Array(arr));
    doc
}

pub fn probes_from_table(doc: &Table) -> Vec<ProbeRow> {
    match doc.get("probe") {
        Some(Value::Array(rows)) => rows
            .iter()
            .filter_map(|row| {
                let t = row.as_table()?;
                Some(ProbeRow {
                    verb: t.get("verb")?.as_str()?.to_string(),
                    rung: str_or_empty(t, "rung"),
                    oracle: str_or_empty(t, "oracle"),
                    runs: int_or_zero(t, "runs") as u32,
                    effect_ok: int_or_zero(t, "effect_ok") as u32,
                    total_ms: int_or_zero(t, "total_ms") as u64,
                })
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// One Battery re-verification row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatteryRow {
    pub verb: String,
    pub ok: bool,
    pub detail: String,
}

pub fn battery_to_table(rows: &[BatteryRow]) -> Table {
    let mut doc = Table::new();
    doc.insert("schema".into(), Value::Integer(1));
    let arr = rows
        .iter()
        .map(|r| {
            let mut t = Table::new();
            t.insert("verb".into(), Value::String(r.verb.clone()));
            t.insert("ok".into(), Value::Boolean(r.ok));
            t.insert("detail".into(), Value::String(r.detail.clone()));
            Value::Table(t)
        })
        .collect();
    doc.insert("check".into(), Value::Array(arr));
    doc
}

pub fn battery_from_table(doc: &Table) -> Vec<BatteryRow> {
    match doc.get("check") {
        Some(Value::Array(rows)) => rows
            .iter()
            .filter_map(|row| {
                let t = row.as_table()?;
                Some(BatteryRow {
                    verb: t.get("verb")?.as_str()?.to_string(),
                    ok: t.get("ok").and_then(Value::as_bool).unwrap_or(false),
                    detail: str_or_empty(t, "detail"),
                })
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// tmp → rename write, mirroring the registry's document law.
pub fn write_table(path: &Path, doc: &Table) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("{} has no parent", path.display()))?;
    fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    let text =
        toml::to_string_pretty(doc).map_err(|e| format!("serialize {}: {e}", path.display()))?;
    let tmp = path.with_extension("toml.tmp");
    fs::write(&tmp, text).map_err(|e| format!("write {}: {e}", tmp.display()))?;
    fs::rename(&tmp, path).map_err(|e| format!("rename {}: {e}", path.display()))?;
    Ok(())
}

pub fn read_table(path: &Path) -> Result<Table, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    text.parse().map_err(|e| format!("{}: {e}", path.display()))
}

fn str_or_empty(t: &Table, key: &str) -> String {
    t.get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn str_array(t: &Table, key: &str) -> Vec<String> {
    match t.get(key) {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        _ => Vec::new(),
    }
}

fn int_or_zero(t: &Table, key: &str) -> i64 {
    t.get(key).and_then(Value::as_integer).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchors_round_trip() {
        let anchors = Anchors {
            atspi_name: "gnome-calculator".into(),
            minted_ms: 812,
            seed_control: Some("7".into()),
            nodes: vec![
                AnchorNode {
                    path: "0/1/2".into(),
                    role: "push button".into(),
                    name: "7".into(),
                    actions: vec!["click".into()],
                },
                AnchorNode {
                    path: "0/1/3".into(),
                    role: "label".into(),
                    name: "display".into(),
                    actions: vec![],
                },
            ],
        };
        let back = Anchors::from_table(&anchors.to_table()).unwrap();
        assert_eq!(back, anchors);
    }

    #[test]
    fn probes_round_trip_and_reliability_is_measured() {
        let rows = vec![ProbeRow {
            verb: "move".into(),
            rung: "r1".into(),
            oracle: "geometry-delta".into(),
            runs: 3,
            effect_ok: 2,
            total_ms: 90,
        }];
        let back = probes_from_table(&probes_to_table(&rows));
        assert_eq!(back, rows);
        assert!((back[0].reliability() - 2.0 / 3.0).abs() < 1e-9);
        let zero = ProbeRow {
            runs: 0,
            ..rows[0].clone()
        };
        assert_eq!(zero.reliability(), 0.0);
    }

    #[test]
    fn battery_round_trip() {
        let rows = vec![BatteryRow {
            verb: "focus".into(),
            ok: true,
            detail: "effect_observed".into(),
        }];
        assert_eq!(battery_from_table(&battery_to_table(&rows)), rows);
    }

    #[test]
    fn write_read_table_never_leaves_tmp() {
        let dir = std::env::temp_dir().join(format!(
            "dpl-foundry-artifact-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = dir.join("probes.toml");
        write_table(&path, &probes_to_table(&[])).unwrap();
        assert!(path.is_file());
        assert!(!path.with_extension("toml.tmp").exists());
        assert!(probes_from_table(&read_table(&path).unwrap()).is_empty());
        let _ = fs::remove_dir_all(dir);
    }
}
