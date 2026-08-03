//! The named Imagine generation adapter (ADR-011).
//!
//! One compiled-in endpoint, one operator-owned key read at call time from
//! `XAI_API_KEY` (never stored, never logged, never part of an error), one
//! in-flight request driven from a worker thread by the app coordinator.
//! Generated artifacts land in the Ring-0 draft cache
//! (`.userdata/cache/imagine/`) — never in `content/` — until the operator
//! explicitly promotes or discards them. Unsaved drafts expire after 30
//! days behind the consent policy in `imagine-policy.json`.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const IMAGE_MODEL: &str = "grok-imagine-image-quality";
pub const IMAGES_ENDPOINT: &str = "https://api.x.ai/v1/images/generations";
/// Unsaved drafts older than this are flush candidates.
pub const RETENTION_MS: u64 = 30 * 24 * 60 * 60 * 1000;
const INDEX_FILE: &str = "index.json";
const POLICY_FILE: &str = "imagine-policy.json";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(90);
const MAX_IMAGE_BYTES: usize = 32 * 1024 * 1024;

/// The operator's key, read from the session environment at call time.
/// DevPULSE never writes it anywhere; absence keeps the feature inert.
pub fn resolve_api_key() -> Option<String> {
    std::env::var("XAI_API_KEY")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationDraft {
    pub id: String,
    pub prompt: String,
    pub model: String,
    pub created_ms: u64,
    /// Artifact file name inside the cache directory.
    pub file: String,
}

impl GenerationDraft {
    pub fn expired(&self, now_ms: u64) -> bool {
        now_ms.saturating_sub(self.created_ms) >= RETENTION_MS
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct CacheIndex {
    drafts: Vec<GenerationDraft>,
}

/// The Ring-0 draft cache. Fully regenerable by design: it lives under
/// `.userdata/cache/`, which the userdata README already declares safe to
/// delete.
#[derive(Debug)]
pub struct ImagineCache {
    dir: PathBuf,
    drafts: Vec<GenerationDraft>,
    next_discriminator: u64,
}

impl ImagineCache {
    pub fn open(dir: impl Into<PathBuf>) -> Self {
        let dir = dir.into();
        let _ = fs::create_dir_all(&dir);
        let drafts = fs::read(dir.join(INDEX_FILE))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<CacheIndex>(&bytes).ok())
            .map(|index| index.drafts)
            .unwrap_or_default();
        // Drop index rows whose artifact vanished (the cache is deletable).
        let drafts = drafts
            .into_iter()
            .filter(|draft| dir.join(&draft.file).is_file())
            .collect();
        Self {
            dir,
            drafts,
            next_discriminator: 0,
        }
    }

    pub fn open_default() -> Self {
        Self::open(
            dpl_engine_capture::userdata_root()
                .join("cache")
                .join("imagine"),
        )
    }

    pub fn drafts(&self) -> &[GenerationDraft] {
        &self.drafts
    }

    pub fn artifact_path(&self, draft: &GenerationDraft) -> PathBuf {
        self.dir.join(&draft.file)
    }

    pub fn add(
        &mut self,
        prompt: &str,
        model: &str,
        bytes: &[u8],
        now_ms: u64,
    ) -> Result<GenerationDraft, String> {
        let extension = sniff_extension(bytes);
        let id = format!("draft-{now_ms}-{}", self.next_discriminator);
        self.next_discriminator += 1;
        let file = format!("{id}.{extension}");
        let path = self.dir.join(&file);
        fs::write(&path, bytes).map_err(|error| format!("draft could not be written: {error}"))?;
        let draft = GenerationDraft {
            id,
            prompt: prompt.to_owned(),
            model: model.to_owned(),
            created_ms: now_ms,
            file,
        };
        self.drafts.push(draft.clone());
        self.save_index()?;
        Ok(draft)
    }

    /// Removes a draft and its artifact — used by both the explicit Ignore
    /// gesture and by promotion (after the library copy has committed).
    pub fn remove(&mut self, id: &str) -> Result<(), String> {
        let Some(position) = self.drafts.iter().position(|draft| draft.id == id) else {
            return Err(format!("unknown draft {id}"));
        };
        let draft = self.drafts.remove(position);
        let path = self.dir.join(&draft.file);
        if path.is_file() {
            fs::remove_file(&path)
                .map_err(|error| format!("draft artifact could not be removed: {error}"))?;
        }
        self.save_index()
    }

    pub fn get(&self, id: &str) -> Option<&GenerationDraft> {
        self.drafts.iter().find(|draft| draft.id == id)
    }

    pub fn retention_due(&self, now_ms: u64) -> bool {
        self.drafts.iter().any(|draft| draft.expired(now_ms))
    }

    /// Removes every expired draft; returns how many were flushed.
    pub fn flush_expired(&mut self, now_ms: u64) -> usize {
        let expired: Vec<String> = self
            .drafts
            .iter()
            .filter(|draft| draft.expired(now_ms))
            .map(|draft| draft.id.clone())
            .collect();
        let mut flushed = 0;
        for id in expired {
            if self.remove(&id).is_ok() {
                flushed += 1;
            }
        }
        flushed
    }

    fn save_index(&self) -> Result<(), String> {
        let index = CacheIndex {
            drafts: self.drafts.clone(),
        };
        let bytes = serde_json::to_vec_pretty(&index)
            .map_err(|error| format!("draft index could not be encoded: {error}"))?;
        let temporary = self.dir.join(format!("{INDEX_FILE}.tmp"));
        fs::write(&temporary, bytes)
            .map_err(|error| format!("draft index could not be written: {error}"))?;
        fs::rename(&temporary, self.dir.join(INDEX_FILE))
            .map_err(|error| format!("draft index could not be committed: {error}"))
    }
}

/// PNG/JPEG magic sniffing; unknown bytes default to PNG.
pub fn sniff_extension(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "jpg"
    } else if bytes.starts_with(&[0x52, 0x49, 0x46, 0x46]) {
        "webp"
    } else {
        "png"
    }
}

/// Flush-consent policy. `auto_flush` is the persisted "never ask again".
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImaginePolicy {
    #[serde(default)]
    pub auto_flush: bool,
}

impl ImaginePolicy {
    pub fn load(userdata: &Path) -> Self {
        fs::read(userdata.join(POLICY_FILE))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, userdata: &Path) -> io::Result<()> {
        let bytes = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        let temporary = userdata.join(format!("{POLICY_FILE}.tmp"));
        fs::write(&temporary, bytes)?;
        fs::rename(temporary, userdata.join(POLICY_FILE))
    }
}

pub trait ImagineClient: std::fmt::Debug + Send {
    /// Blocking; the app runs this on a bounded worker thread, never the
    /// UI thread. Returns the decoded image bytes.
    fn generate_image(&self, api_key: &str, prompt: &str) -> Result<Vec<u8>, String>;
}

#[derive(Debug, Default)]
pub struct SystemImagineClient;

pub fn image_request_json(prompt: &str) -> serde_json::Value {
    serde_json::json!({
        "model": IMAGE_MODEL,
        "prompt": prompt,
        "n": 1,
        "response_format": "b64_json",
    })
}

pub fn parse_image_response(body: &str) -> Result<Vec<u8>, String> {
    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|_| "the generation response was not valid JSON".to_owned())?;
    let encoded = value
        .get("data")
        .and_then(|data| data.get(0))
        .and_then(|first| first.get("b64_json"))
        .and_then(|encoded| encoded.as_str())
        .ok_or_else(|| "the generation response carried no image".to_owned())?;
    let bytes = data_encoding::BASE64
        .decode(encoded.as_bytes())
        .map_err(|_| "the generation payload was not valid base64".to_owned())?;
    if bytes.is_empty() || bytes.len() > MAX_IMAGE_BYTES {
        return Err("the generation payload size is outside bounds".to_owned());
    }
    Ok(bytes)
}

impl ImagineClient for SystemImagineClient {
    fn generate_image(&self, api_key: &str, prompt: &str) -> Result<Vec<u8>, String> {
        // ureq 3: AgentBuilder/timeout/send_string/Error::Status|Transport
        // are gone. config_builder + send + StatusCode preserve the same
        // operator-facing strings (no key ever lands in an error path).
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(REQUEST_TIMEOUT))
            .build()
            .into();
        let response = agent
            .post(IMAGES_ENDPOINT)
            .header("Authorization", &format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .send(image_request_json(prompt).to_string());
        let mut response = match response {
            Ok(response) => response,
            Err(ureq::Error::StatusCode(code)) => {
                return Err(format!("the generation service answered HTTP {code}"));
            }
            Err(_) => {
                return Err("the generation service could not be reached".to_owned());
            }
        };
        let body = response
            .body_mut()
            .read_to_string()
            .map_err(|_| "the generation response could not be read".to_owned())?;
        parse_image_response(&body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_cache(name: &str) -> ImagineCache {
        let dir = std::env::temp_dir().join(format!(
            "dpl-imagine-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.subsec_nanos())
                .unwrap_or_default()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        ImagineCache::open(dir)
    }

    #[test]
    fn cache_roundtrips_add_list_remove() {
        let mut cache = temp_cache("roundtrip");
        let draft = cache
            .add("a red panda", IMAGE_MODEL, b"\x89PNG-not-really", 1_000)
            .expect("add draft");
        assert_eq!(cache.drafts().len(), 1);
        assert!(cache.artifact_path(&draft).is_file());
        assert!(draft.file.ends_with(".png"));

        let reopened = ImagineCache::open(cache.dir.clone());
        assert_eq!(reopened.drafts(), cache.drafts());

        cache.remove(&draft.id).expect("remove draft");
        assert!(cache.drafts().is_empty());
        assert!(!cache.artifact_path(&draft).is_file());
        assert!(cache.remove(&draft.id).is_err());
    }

    #[test]
    fn retention_expires_at_thirty_days_and_flushes_only_expired() {
        let mut cache = temp_cache("retention");
        cache.add("old", IMAGE_MODEL, b"old", 0).expect("old draft");
        cache
            .add("fresh", IMAGE_MODEL, b"fresh", RETENTION_MS / 2)
            .expect("fresh draft");
        assert!(!cache.retention_due(RETENTION_MS - 1));
        assert!(cache.retention_due(RETENTION_MS));
        assert_eq!(cache.flush_expired(RETENTION_MS), 1);
        assert_eq!(cache.drafts().len(), 1);
        assert_eq!(cache.drafts()[0].prompt, "fresh");
    }

    #[test]
    fn missing_artifacts_are_dropped_on_open() {
        let mut cache = temp_cache("orphans");
        let draft = cache
            .add("vanishing", IMAGE_MODEL, b"bytes", 5)
            .expect("draft");
        std::fs::remove_file(cache.artifact_path(&draft)).expect("delete artifact");
        let reopened = ImagineCache::open(cache.dir.clone());
        assert!(reopened.drafts().is_empty());
    }

    #[test]
    fn policy_roundtrips_and_defaults_closed() {
        let dir = std::env::temp_dir().join(format!("dpl-imagine-policy-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::remove_file(dir.join(POLICY_FILE));
        assert_eq!(
            ImaginePolicy::load(&dir),
            ImaginePolicy { auto_flush: false }
        );
        let policy = ImaginePolicy { auto_flush: true };
        policy.save(&dir).expect("save policy");
        assert_eq!(ImaginePolicy::load(&dir), policy);
    }

    #[test]
    fn request_json_is_exactly_the_bounded_shape() {
        let request = image_request_json("draw a lighthouse");
        assert_eq!(request["model"], IMAGE_MODEL);
        assert_eq!(request["prompt"], "draw a lighthouse");
        assert_eq!(request["n"], 1);
        assert_eq!(request["response_format"], "b64_json");
        assert_eq!(
            request.as_object().map(|object| object.len()),
            Some(4),
            "no undocumented fields may ride along"
        );
    }

    #[test]
    fn response_parsing_is_fail_closed() {
        let encoded = data_encoding::BASE64.encode(b"image-bytes");
        let body = format!("{{\"data\":[{{\"b64_json\":\"{encoded}\"}}]}}");
        assert_eq!(parse_image_response(&body).expect("bytes"), b"image-bytes");
        assert!(parse_image_response("not json").is_err());
        assert!(parse_image_response("{\"data\":[]}").is_err());
        assert!(parse_image_response("{\"data\":[{\"b64_json\":\"!!!\"}]}").is_err());
        assert!(parse_image_response("{\"data\":[{\"b64_json\":\"\"}]}").is_err());
    }

    #[test]
    fn sniffing_covers_the_documented_formats() {
        assert_eq!(sniff_extension(&[0xFF, 0xD8, 0xFF, 0xE0]), "jpg");
        assert_eq!(sniff_extension(b"RIFFxxxxWEBP"), "webp");
        assert_eq!(sniff_extension(b"\x89PNG\r\n"), "png");
        assert_eq!(sniff_extension(b""), "png");
    }
}
