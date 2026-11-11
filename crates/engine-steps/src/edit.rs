//! Immutable guide review revisions: prune/reorder/caption without changing
//! the source. All regular assets are copied, including pruned frames.
//!
//! This bounded editor accepts flat, unexported guide folders. Static ancestry,
//! filename and link/reparse checks reject unsafe inputs; these checks are not
//! a defense against a hostile concurrent process replacing filesystem paths.
//! A pending marker remains on failed revisions. Readers must ignore it until
//! it is renamed to the completion marker. No source content is deleted.

use std::collections::{HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

const PENDING: &str = ".guide-revision-pending";
const COMPLETE: &str = ".guide-revision-complete";
const STAGED_MANIFEST: &str = "guide.json.pending";

/// An original step identity and its reviewed caption. Every other field must
/// match the original; the vector order selects and reorders the kept steps.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GuideStepEdit {
    pub order: u32,
    pub file: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_file: Option<String>,
    #[serde(default)]
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub click: Option<[f64; 2]>,
    #[serde(default)]
    pub app: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
    #[serde(default)]
    pub bytes: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clip: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clip_in: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clip_out: Option<f32>,
}

/// A newly published sibling revision. The source path and files are untouched.
#[derive(Debug)]
pub struct GuideRevision {
    pub dir: PathBuf,
    pub steps: u32,
}

// Value's normal decoder accepts duplicate keys by dropping earlier values.
// Refuse ambiguity at every depth before preserving extension metadata.
struct UniqueValue(Value);

impl<'de> Deserialize<'de> for UniqueValue {
    fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        use serde::de::{Error, MapAccess, SeqAccess, Visitor};
        struct UniqueVisitor;
        impl<'de> Visitor<'de> for UniqueVisitor {
            type Value = UniqueValue;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("JSON with unique object keys")
            }
            fn visit_bool<E: Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Bool(v)))
            }
            fn visit_i64<E: Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::from(v)))
            }
            fn visit_u64<E: Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::from(v)))
            }
            fn visit_f64<E: Error>(self, v: f64) -> Result<Self::Value, E> {
                serde_json::Number::from_f64(v)
                    .map(|v| UniqueValue(Value::Number(v)))
                    .ok_or_else(|| E::custom("non-finite number"))
            }
            fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::from(v)))
            }
            fn visit_string<E: Error>(self, v: String) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::from(v)))
            }
            fn visit_unit<E: Error>(self) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(value) = seq.next_element::<UniqueValue>()? {
                    values.push(value.0);
                }
                Ok(UniqueValue(Value::Array(values)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some(key) = map.next_key::<String>()? {
                    if values.contains_key(&key) {
                        return Err(A::Error::custom(format!("duplicate object key {key:?}")));
                    }
                    values.insert(key, map.next_value::<UniqueValue>()?.0);
                }
                Ok(UniqueValue(Value::Object(values)))
            }
        }
        decoder.deserialize_any(UniqueVisitor)
    }
}

fn safe_name(name: &str) -> Result<(), String> {
    let stem = name.split('.').next().unwrap_or_default().trim_end();
    let upper = stem.to_uppercase();
    let device = matches!(
        upper.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || ["COM", "LPT"].iter().any(|prefix| {
        upper.strip_prefix(prefix).is_some_and(|tail| {
            matches!(
                tail,
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        })
    });
    if name.is_empty()
        || name == "."
        || name.contains("..")
        || name.ends_with(['.', ' '])
        || name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
        || device
    {
        return Err(format!("unsafe guide filename: {name:?}"));
    }
    Ok(())
}

fn is_link(meta: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        meta.file_type().is_symlink()
    }
}

/// Inspect before canonicalization, which would hide symlinks/junctions.
fn check_ancestry(dir: &Path) -> Result<(), String> {
    let absolute = if dir.is_absolute() {
        dir.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| format!("current directory: {e}"))?
            .join(dir)
    };
    let mut cursor = PathBuf::new();
    for part in absolute.components() {
        if matches!(part, Component::ParentDir) {
            return Err("guide folder must not contain parent-directory components".into());
        }
        cursor.push(part.as_os_str());
        if matches!(part, Component::Prefix(_)) {
            continue;
        }
        let meta = fs::symlink_metadata(&cursor)
            .map_err(|e| format!("inspect {}: {e}", cursor.display()))?;
        if is_link(&meta) || !meta.is_dir() {
            return Err(format!(
                "guide ancestry must contain ordinary directories: {}",
                cursor.display()
            ));
        }
    }
    Ok(())
}

fn regular_file(path: &Path) -> Result<(), String> {
    let meta =
        fs::symlink_metadata(path).map_err(|e| format!("inspect {}: {e}", path.display()))?;
    if is_link(&meta) || !meta.is_file() {
        return Err(format!(
            "guide assets must be regular files, not links or nested folders: {}",
            path.display()
        ));
    }
    Ok(())
}

fn media_reference(dir: &Path, name: &str) -> Result<(), String> {
    safe_name(name)?;
    if [PENDING, COMPLETE, STAGED_MANIFEST, "guide.json"]
        .iter()
        .any(|reserved| name.eq_ignore_ascii_case(reserved))
    {
        return Err(format!("reserved guide file cannot be media: {name:?}"));
    }
    regular_file(&dir.join(name))
}

fn step_references(dir: &Path, step: &GuideStepEdit) -> Result<(), String> {
    if !(step.kind == "replay" && step.file.is_empty()) {
        media_reference(dir, &step.file)?;
    }
    for reference in [&step.result_file, &step.clip].into_iter().flatten() {
        media_reference(dir, reference)?;
    }
    Ok(())
}

fn source_files(dir: &Path) -> Result<Vec<String>, String> {
    let mut names = Vec::new();
    let mut folded = HashSet::new();
    for entry in fs::read_dir(dir).map_err(|e| format!("read guide folder: {e}"))? {
        let entry = entry.map_err(|e| format!("read guide entry: {e}"))?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "guide filenames must be UTF-8".to_string())?;
        safe_name(&name)?;
        regular_file(&entry.path())?;
        if !folded.insert(name.to_lowercase()) {
            return Err("guide filenames must be distinct without case differences".into());
        }
        if name.eq_ignore_ascii_case(PENDING) {
            return Err(
                "this guide revision is incomplete; retry from its unchanged source".into(),
            );
        }
        if name.eq_ignore_ascii_case(STAGED_MANIFEST) {
            return Err("guide folder contains reserved staging file guide.json.pending".into());
        }
        if name.eq_ignore_ascii_case("guide.html") {
            return Err(
                "this is an exported guide; edit its source guide, then export again".into(),
            );
        }
        if (name.eq_ignore_ascii_case("guide.json") && name != "guide.json")
            || (name.eq_ignore_ascii_case(COMPLETE) && name != COMPLETE)
        {
            return Err(format!(
                "reserved guide filename has incompatible case: {name:?}"
            ));
        }
        names.push(name);
    }
    names.sort();
    if !names.iter().any(|n| n == "guide.json") {
        return Err("this artifact has no guide.json".into());
    }
    Ok(names)
}

fn revision_manifest(dir: &Path, steps: &[GuideStepEdit]) -> Result<Vec<u8>, String> {
    let raw = fs::read(dir.join("guide.json")).map_err(|e| format!("read guide.json: {e}"))?;
    let mut manifest = serde_json::from_slice::<UniqueValue>(&raw)
        .map_err(|e| format!("parse guide.json: {e}"))?
        .0;
    let root = manifest
        .as_object_mut()
        .ok_or("guide manifest must be an object")?;
    if root.get("kind").and_then(Value::as_str) != Some("guide")
        || root.get("name").and_then(Value::as_str).is_none()
    {
        return Err("guide manifest requires kind 'guide' and a string name".into());
    }
    for key in ["guideVideo", "replayGif"] {
        match root.get(key) {
            None | Some(Value::Null) => {}
            Some(Value::String(value)) if value.is_empty() => {}
            Some(_) => {
                return Err(
                    "this is an exported guide; edit its source guide, then export again".into(),
                )
            }
        }
    }
    match root.get("session") {
        None | Some(Value::Null) => {}
        Some(Value::Object(session)) => {
            match session.get("video") {
                None | Some(Value::Null) => {}
                Some(Value::String(video)) => media_reference(dir, video)?,
                Some(_) => return Err("session.video must be a filename or null".into()),
            }
            // Match the reader's SessionRef/Tick shape without reconstructing
            // either object: extension fields must survive this edit.
            if let Some(events) = session.get("events") {
                let events = events
                    .as_array()
                    .ok_or("session.events must be an array when present")?;
                for (index, event) in events.iter().enumerate() {
                    if !event.is_object()
                        || event.get("ms").and_then(Value::as_u64).is_none()
                        || event.get("label").and_then(Value::as_str).is_none()
                    {
                        return Err(format!(
                            "session.events[{index}] must be an object with unsigned integer ms and string label"
                        ));
                    }
                }
            }
        }
        Some(_) => return Err("session must be an object or null".into()),
    }
    let originals = root
        .get("steps")
        .and_then(Value::as_array)
        .ok_or("guide manifest requires a steps array")?;
    let mut by_order = HashMap::new();
    for raw_step in originals {
        let parsed: GuideStepEdit = serde_json::from_value(raw_step.clone())
            .map_err(|e| format!("invalid original guide step: {e}"))?;
        if parsed.order == 0 || by_order.contains_key(&parsed.order) {
            return Err("original steps require unique positive order identities".into());
        }
        step_references(dir, &parsed)?;
        by_order.insert(parsed.order, (parsed, raw_step));
    }
    let mut used = HashSet::new();
    let mut edited = Vec::with_capacity(steps.len());
    let mut total_bytes = 0usize;
    for (index, step) in steps.iter().enumerate() {
        step_references(dir, step)?;
        if !used.insert(step.order) {
            return Err(format!("duplicate submitted step identity: {}", step.order));
        }
        let (original, raw_step) = by_order
            .get(&step.order)
            .ok_or_else(|| format!("unknown original step identity: {}", step.order))?;
        let mut comparable = original.clone();
        comparable.caption = step.caption.clone();
        if &comparable != step {
            return Err(format!(
                "step {} may only change its caption, position or kept state",
                step.order
            ));
        }
        let mut merged = (*raw_step).clone();
        let fields = merged
            .as_object_mut()
            .ok_or("original step must be an object")?;
        fields.insert("order".into(), Value::from(index + 1));
        match &step.caption {
            Some(caption) => {
                fields.insert("caption".into(), Value::String(caption.clone()));
            }
            None => {
                fields.remove("caption");
            }
        }
        total_bytes = total_bytes
            .checked_add(original.bytes)
            .ok_or("guide totalBytes overflow")?;
        edited.push(merged);
    }
    root.insert("steps".into(), Value::Array(edited));
    root.insert("stepCount".into(), Value::from(steps.len()));
    root.insert("totalBytes".into(), Value::from(total_bytes));
    let mut bytes =
        serde_json::to_vec_pretty(&manifest).map_err(|e| format!("guide manifest: {e}"))?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn create_synced(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("create {}: {e}", path.display()))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|e| format!("write {}: {e}", path.display()))
}

fn reserve_revision(dir: &Path) -> Result<PathBuf, String> {
    let parent = dir.parent().ok_or("guide must have a parent folder")?;
    let name = dir
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("guide folder name must be UTF-8")?;
    safe_name(name)?;
    for number in 1..=10_000 {
        let revision = parent.join(format!("{name}-revision-{number}"));
        match fs::create_dir(&revision) {
            Ok(()) => return Ok(revision),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(format!("reserve guide revision: {e}")),
        }
    }
    Err("no free guide revision name within 10000 attempts".into())
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Checkpoint {
    Copy,
    ManifestRename,
    Complete,
}

/// Save the kept steps in a fresh sibling revision. The source, pruned media and
/// extension metadata survive. Failures retain an unpublished pending folder;
/// retries reserve another name. Returned paths retain the caller's path form.
pub fn update_guide(dir: &Path, steps: Vec<GuideStepEdit>) -> Result<GuideRevision, String> {
    update_with_hook(dir, steps, |_| Ok(()))
}

fn update_with_hook(
    dir: &Path,
    steps: Vec<GuideStepEdit>,
    mut checkpoint: impl FnMut(Checkpoint) -> Result<(), String>,
) -> Result<GuideRevision, String> {
    if steps.is_empty() {
        return Err("a guide needs at least one kept step".into());
    }
    let count = u32::try_from(steps.len()).map_err(|_| "too many guide steps")?;
    check_ancestry(dir)?;
    let files = source_files(dir)?;
    let manifest = revision_manifest(dir, &steps)?;
    let revision = reserve_revision(dir)?;
    let result = (|| {
        create_synced(&revision.join(PENDING), b"Guide revision is incomplete.\n")?;
        for name in files
            .iter()
            .filter(|name| name.as_str() != "guide.json" && name.as_str() != COMPLETE)
        {
            checkpoint(Checkpoint::Copy)?;
            regular_file(&dir.join(name))?;
            let mut source = File::open(dir.join(name)).map_err(|e| format!("open {name}: {e}"))?;
            let mut target = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(revision.join(name))
                .map_err(|e| format!("create revision asset {name}: {e}"))?;
            std::io::copy(&mut source, &mut target)
                .and_then(|_| target.sync_all())
                .map_err(|e| format!("copy revision asset {name}: {e}"))?;
        }
        create_synced(&revision.join(STAGED_MANIFEST), &manifest)?;
        checkpoint(Checkpoint::ManifestRename)?;
        fs::rename(revision.join(STAGED_MANIFEST), revision.join("guide.json"))
            .map_err(|e| format!("publish revision manifest: {e}"))?;
        checkpoint(Checkpoint::Complete)?;
        fs::rename(revision.join(PENDING), revision.join(COMPLETE))
            .map_err(|e| format!("complete guide revision: {e}"))?;
        Ok(())
    })();
    result.map_err(|e: String| {
        format!(
            "{e}; unfinished revision retained at {}",
            revision.display()
        )
    })?;
    Ok(GuideRevision {
        dir: revision,
        steps: count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);

    struct Fixture {
        root: PathBuf,
        dir: PathBuf,
    }
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "dpl-guide-revision-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&root).unwrap();
            let dir = root.join("source");
            fs::create_dir(&dir).unwrap();
            for name in ["one.webp", "done.webp", "two.webp", "clip.mp4", "extra.txt"] {
                fs::write(dir.join(name), name.as_bytes()).unwrap();
            }
            let mut steps = vec![
                serde_json::to_value(edit(1, "one.webp")).unwrap(),
                serde_json::to_value(edit(2, "two.webp")).unwrap(),
            ];
            steps[0]["resultFile"] = Value::from("done.webp");
            steps[0]["future"] = serde_json::json!({"nested": [1, "retained"]});
            steps[1]["caption"] = Value::from("old caption");
            let fixture = Self { root, dir };
            fixture.write(&serde_json::json!({
                "kind": "guide", "name": "Original", "stepCount": 2, "totalBytes": 2,
                "recordedAt": "unchanged", "extension": {"data": ["kept"]}, "steps": steps
            }));
            fixture
        }
        fn manifest(&self) -> Value {
            read_manifest(&self.dir)
        }
        fn write(&self, value: &Value) {
            fs::write(
                self.dir.join("guide.json"),
                serde_json::to_vec_pretty(value).unwrap(),
            )
            .unwrap();
        }
        fn steps(&self) -> Vec<GuideStepEdit> {
            serde_json::from_value(self.manifest()["steps"].clone()).unwrap()
        }
        fn snapshot(&self) -> Vec<(String, Vec<u8>)> {
            let mut result: Vec<_> = fs::read_dir(&self.dir)
                .unwrap()
                .map(|entry| {
                    let entry = entry.unwrap();
                    (
                        entry.file_name().into_string().unwrap(),
                        fs::read(entry.path()).unwrap(),
                    )
                })
                .collect();
            result.sort();
            result
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
    fn read_manifest(dir: &Path) -> Value {
        serde_json::from_slice(&fs::read(dir.join("guide.json")).unwrap()).unwrap()
    }
    fn edit(order: u32, file: &str) -> GuideStepEdit {
        GuideStepEdit {
            order,
            file: file.into(),
            result_file: None,
            action: "click".into(),
            click: Some([0.25, 0.5]),
            app: "Test".into(),
            title: "Window".into(),
            width: 10,
            height: 10,
            bytes: 1,
            caption: None,
            kind: String::new(),
            clip: None,
            clip_in: None,
            clip_out: None,
        }
    }
    fn no_revision(f: &Fixture) {
        assert_eq!(fs::read_dir(&f.root).unwrap().count(), 1);
    }

    #[test]
    fn revisions_preserve_source_assets_and_unknown_metadata() {
        let f = Fixture::new();
        let before = f.snapshot();
        let original = f.manifest();
        let mut steps = f.steps();
        steps.reverse();
        steps[0].caption = None;
        steps[1].caption = Some("new caption".into());
        let revision = update_guide(&f.dir, steps).unwrap();
        assert_eq!(revision.steps, 2);
        assert_eq!(before, f.snapshot());
        let manifest = read_manifest(&revision.dir);
        assert_eq!(manifest["extension"], original["extension"]);
        assert_eq!(manifest["recordedAt"], original["recordedAt"]);
        assert_eq!(
            manifest["steps"][1]["future"],
            original["steps"][0]["future"]
        );
        assert!(manifest["steps"][0].get("caption").is_none());
        assert_eq!(manifest["steps"][1]["caption"], "new caption");
        assert_eq!(manifest["steps"][0]["order"], 1);
        assert!(revision.dir.join(COMPLETE).is_file());
        assert!(!revision.dir.join(PENDING).exists());
        for (name, bytes) in &before {
            if name != "guide.json" {
                assert_eq!(*bytes, fs::read(revision.dir.join(name)).unwrap());
            }
        }
        let mut kept: Vec<GuideStepEdit> =
            serde_json::from_value(manifest["steps"].clone()).unwrap();
        kept.truncate(1);
        let second = update_guide(&revision.dir, kept).unwrap();
        assert!(
            second.dir.join("one.webp").is_file(),
            "prune removes manifest rows only"
        );
        assert!(
            revision.dir.join(COMPLETE).is_file(),
            "original completion is retained"
        );
        assert_eq!(read_manifest(&second.dir)["stepCount"], 1);
    }

    #[test]
    fn replay_caption_preserves_raw_timing_and_session_metadata() {
        let f = Fixture::new();
        let mut raw = f.manifest();
        raw["steps"][1]["kind"] = Value::from("replay");
        raw["steps"][1]["file"] = Value::from("");
        raw["steps"][1]["clip"] = Value::from("clip.mp4");
        raw["steps"][1]["clipIn"] = Value::from(1.234567891);
        raw["steps"][1]["clipOut"] = Value::from(2.5);
        raw["session"] = serde_json::json!({
            "video":"clip.mp4",
            "events":[{"ms":0,"label":"start","extension":{"retained":true}}],
            "extension":42
        });
        f.write(&raw);
        let before = f.snapshot();
        let mut steps = f.steps();
        steps[1].caption = Some("watch the drag".into());
        let revision = update_guide(&f.dir, steps).unwrap();
        let revised = read_manifest(&revision.dir);
        assert_eq!(revised["steps"][1]["clipIn"], raw["steps"][1]["clipIn"]);
        assert_eq!(revised["session"], raw["session"]);
        assert_eq!(before, f.snapshot());
    }

    #[test]
    fn malformed_session_events_refuse_before_reserving_revision() {
        let f = Fixture::new();
        let steps = f.steps();
        let original = f.manifest();
        for events in [
            Value::from("invalid"),
            Value::Null,
            serde_json::json!({}),
            serde_json::json!([null]),
            serde_json::json!([{}]),
            serde_json::json!([{"ms":0}]),
            serde_json::json!([{"label":"missing time"}]),
            serde_json::json!([{"ms":-1,"label":"negative"}]),
            serde_json::json!([{"ms":1.5,"label":"fractional"}]),
            serde_json::json!([{"ms":"0","label":"string time"}]),
            serde_json::json!([{"ms":0,"label":null}]),
            serde_json::json!([{"ms":0,"label":1}]),
        ] {
            let mut raw = original.clone();
            raw["session"] = serde_json::json!({"events":events,"extension":"kept"});
            f.write(&raw);
            let before = f.snapshot();
            assert!(update_guide(&f.dir, steps.clone()).is_err(), "{events}");
            assert_eq!(before, f.snapshot());
            no_revision(&f);
        }
        let mut raw = original;
        raw["session"] = serde_json::json!({"extension":"events may be absent"});
        f.write(&raw);
        let revision = update_guide(&f.dir, steps).unwrap();
        assert_eq!(read_manifest(&revision.dir)["session"], raw["session"]);
    }

    #[test]
    fn omitted_malicious_old_reference_never_touches_outside_sentinel() {
        let f = Fixture::new();
        fs::write(f.root.join("sentinel"), b"do not touch").unwrap();
        let submitted = vec![f.steps()[0].clone()];
        let mut raw = f.manifest();
        raw["steps"][1]["file"] = Value::from("../sentinel");
        f.write(&raw);
        let before = f.snapshot();
        assert!(update_guide(&f.dir, submitted).is_err());
        assert_eq!(fs::read(f.root.join("sentinel")).unwrap(), b"do not touch");
        assert_eq!(before, f.snapshot());
        assert_eq!(fs::read_dir(&f.root).unwrap().count(), 2);
    }

    #[test]
    fn unsafe_names_and_all_replay_references_are_refused() {
        for bad in [
            "../outside",
            "x/y",
            "x\\y",
            "C:outside",
            "asset:stream",
            "NUL",
            "COM1.txt",
            "name.",
            "name ",
            "a?b",
            "",
        ] {
            assert!(safe_name(bad).is_err(), "{bad}");
        }
        for key in ["file", "resultFile", "clip"] {
            let f = Fixture::new();
            let mut raw = f.manifest();
            raw["steps"][1]["kind"] = Value::from("replay");
            raw["steps"][1][key] = Value::from("../outside");
            f.write(&raw);
            assert!(
                update_guide(&f.dir, vec![f.steps()[0].clone()]).is_err(),
                "{key}"
            );
            no_revision(&f);
        }
        let f = Fixture::new();
        let mut steps = f.steps();
        steps[0].result_file = Some("../outside".into());
        assert!(update_guide(&f.dir, steps).is_err());
        no_revision(&f);
    }

    #[test]
    fn semantic_substitution_and_duplicate_or_missing_identity_are_refused() {
        let f = Fixture::new();
        let before = f.snapshot();
        let steps = f.steps();
        for field in [
            "file",
            "action",
            "click",
            "app",
            "title",
            "width",
            "height",
            "bytes",
            "kind",
            "clip",
            "clipIn",
            "clipOut",
            "resultFile",
        ] {
            let mut changed = serde_json::to_value(&steps[0]).unwrap();
            changed[field] = match field {
                "file" => Value::from("two.webp"),
                "clip" | "resultFile" => Value::from("clip.mp4"),
                "width" | "height" | "bytes" => Value::from(9),
                "click" => serde_json::json!([0.1, 0.2]),
                "clipIn" | "clipOut" => Value::from(1.0),
                _ => Value::from("changed"),
            };
            let changed: GuideStepEdit = serde_json::from_value(changed).unwrap();
            assert!(update_guide(&f.dir, vec![changed]).is_err(), "{field}");
        }
        assert!(update_guide(&f.dir, vec![steps[0].clone(), steps[0].clone()]).is_err());
        let mut unknown = steps[0].clone();
        unknown.order = 99;
        assert!(update_guide(&f.dir, vec![unknown]).is_err());
        assert!(update_guide(&f.dir, vec![]).is_err());
        assert_eq!(before, f.snapshot());
        no_revision(&f);
        let mut raw = f.manifest();
        raw["steps"][1]["order"] = Value::from(1);
        f.write(&raw);
        assert!(update_guide(&f.dir, vec![steps[0].clone()]).is_err());
        raw["steps"][1].as_object_mut().unwrap().remove("order");
        f.write(&raw);
        assert!(update_guide(&f.dir, vec![steps[0].clone()]).is_err());
        no_revision(&f);
    }

    #[test]
    fn exported_pending_nested_and_reserved_sources_are_refused() {
        for name in ["guide.html", PENDING, STAGED_MANIFEST] {
            let f = Fixture::new();
            fs::write(f.dir.join(name), b"existing").unwrap();
            assert!(update_guide(&f.dir, f.steps()).is_err(), "{name}");
            no_revision(&f);
        }
        for key in ["guideVideo", "replayGif"] {
            let f = Fixture::new();
            let mut raw = f.manifest();
            raw[key] = Value::from("render.mp4");
            f.write(&raw);
            assert!(update_guide(&f.dir, f.steps()).is_err());
            no_revision(&f);
        }
        let f = Fixture::new();
        fs::create_dir(f.dir.join("attachment")).unwrap();
        assert!(update_guide(&f.dir, f.steps()).is_err());
        no_revision(&f);
    }

    #[test]
    fn malformed_and_ambiguous_metadata_refuses_without_creating_revision() {
        let f = Fixture::new();
        let steps = f.steps();
        for raw in [
            r#"{"steps":[],"steps":[]}"#,
            r#"{"extension":{"key":1,"\u006bey":2}}"#,
            r#"{"steps":[{"order":1,"order":2}]}"#,
            r#"{} {}"#,
            r#"[]"#,
        ] {
            fs::write(f.dir.join("guide.json"), raw).unwrap();
            assert!(update_guide(&f.dir, steps.clone()).is_err(), "{raw}");
            assert_eq!(fs::read(f.dir.join("guide.json")).unwrap(), raw.as_bytes());
            no_revision(&f);
        }
    }

    #[test]
    fn collisions_and_injected_failures_keep_pending_and_retry_fresh() {
        for failure in [
            Checkpoint::Copy,
            Checkpoint::ManifestRename,
            Checkpoint::Complete,
        ] {
            let f = Fixture::new();
            let before = f.snapshot();
            let occupied = f.root.join("source-revision-1");
            fs::create_dir(&occupied).unwrap();
            fs::write(occupied.join("sentinel"), b"existing").unwrap();
            let mut copies = 0;
            let error = update_with_hook(&f.dir, f.steps(), |point| {
                if point == Checkpoint::Copy {
                    copies += 1;
                }
                if point == failure && (point != Checkpoint::Copy || copies == 2) {
                    Err("injected write/publication failure".into())
                } else {
                    Ok(())
                }
            })
            .unwrap_err();
            assert!(error.contains("unfinished revision retained"));
            let pending = f.root.join("source-revision-2");
            assert!(pending.join(PENDING).is_file());
            assert!(!pending.join(COMPLETE).exists());
            assert_eq!(
                pending.join("guide.json").exists(),
                failure == Checkpoint::Complete
            );
            assert!(update_guide(&pending, f.steps()).is_err());
            let retry = update_guide(&f.dir, f.steps()).unwrap();
            assert_eq!(retry.dir, f.root.join("source-revision-3"));
            assert_eq!(before, f.snapshot());
            assert_eq!(fs::read(occupied.join("sentinel")).unwrap(), b"existing");
        }
    }

    #[cfg(unix)]
    #[test]
    fn links_in_assets_manifest_and_ancestry_are_refused() {
        use std::os::unix::fs::symlink;
        let f = Fixture::new();
        let steps = f.steps();
        symlink(f.dir.join("one.webp"), f.dir.join("linked.webp")).unwrap();
        assert!(update_guide(&f.dir, steps.clone()).is_err());
        fs::remove_file(f.dir.join("linked.webp")).unwrap();
        let linked = f.root.join("linked");
        symlink(&f.dir, &linked).unwrap();
        assert!(update_guide(&linked, steps.clone()).is_err());
        let manifest = f.root.join("manifest.json");
        fs::copy(f.dir.join("guide.json"), &manifest).unwrap();
        fs::remove_file(f.dir.join("guide.json")).unwrap();
        symlink(manifest, f.dir.join("guide.json")).unwrap();
        assert!(update_guide(&f.dir, steps).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn windows_reparse_directories_are_refused_when_symlinks_available() {
        use std::os::windows::fs::symlink_dir;
        let f = Fixture::new();
        let linked = f.root.join("linked");
        match symlink_dir(&f.dir, &linked) {
            Ok(()) => assert!(update_guide(&linked, f.steps()).is_err()),
            Err(e) if e.raw_os_error() == Some(1314) => {
                eprintln!("symlink fixture unavailable: Windows privilege not held");
            }
            Err(e) => panic!("create symlink fixture: {e}"),
        }
    }
}
