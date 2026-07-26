//! The LANDING POLICY — the shared "land it, organized" model for both houses
//! (docs/plans/guided-landing-charter.md).
//!
//! One typed model, one reusable card (built in the UI layer): PixVix captures
//! (W9) and finished slides (W10) both land THROUGH this — organized, never a
//! dumping ground. The behaviour is an operator TOGGLE: `Auto` files silently
//! by a pattern; `Ask` shows a card PRE-FILLED from that same pattern. One
//! pattern seeds both, so there is one code path and one card.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// The kind of artifact landing — replaces the scattered `"image"`/`"video"`/
/// `"session"`/`"guide"` string literals so a new vehicle can't drift. It is
/// the `<kind>` tier of the organized inbox path `inbox/<bucket>/<kind>/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CaptureKind {
    Image,
    Video,
    Session,
    Guide,
}

impl CaptureKind {
    /// The stable index/wire kind string (the index and the signal fabric
    /// speak this — never rename).
    pub fn as_str(self) -> &'static str {
        match self {
            CaptureKind::Image => "image",
            CaptureKind::Video => "video",
            CaptureKind::Session => "session",
            CaptureKind::Guide => "guide",
        }
    }
}

/// How a capture files: silently by pattern, or via a pre-filled card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LandingMode {
    /// File immediately by the pattern — effortless, never interrupts.
    #[default]
    Auto,
    /// Show the LandingCard (pre-filled from the pattern) on every capture.
    Ask,
}

/// WHERE an artifact lands. The two variants serve the two houses — the reason
/// this type is SHARED: a wiredFRONT library bucket (W9, PREP), or a Workbench
/// episode chapter/slot (W10, BUILD — reserved here, wired by that pass).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Destination {
    /// PREP house (W9): the working-set bucket under the inbox.
    LibraryBucket { bucket: String },
    /// BUILD house (W10, reserved): a finished slide placed into an episode.
    Episode {
        show: String,
        episode: String,
        chapter: String,
        slot: Option<usize>,
    },
}

impl Destination {
    /// The working-set bucket for a PREP landing (`None` for a BUILD one).
    pub fn bucket(&self) -> Option<&str> {
        match self {
            Destination::LibraryBucket { bucket } => Some(bucket),
            Destination::Episode { .. } => None,
        }
    }
}

/// Suggested working-set seed values the bucket picker offers (operator-
/// adjustable). Any new name is just a folder — kebab-forced on landing.
pub const BUCKET_SEEDS: &[&str] = &["devpulse-news", "eons-of-stupidity", "effingai", "scratch"];

/// The values a vehicle supplies to expand a name template. The caller passes
/// `date` (this crate never reads the clock — keeps it pure and testable).
#[derive(Debug, Clone, Default)]
pub struct NameContext<'a> {
    /// Focused app / window-title stem, when the vehicle knows it.
    pub app: Option<&'a str>,
    /// Full window title, when known.
    pub title: Option<&'a str>,
    /// The capture kind string (`CaptureKind::as_str`).
    pub kind: &'a str,
    /// A never-clobber sequence the caller mints.
    pub seq: u32,
    /// Caller-supplied `YYYYMMDD` (or empty).
    pub date: &'a str,
}

/// The pattern: a meaningful name template, a destination, default tags.
/// Extensible — a new key here is inherited by every vehicle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LandingPattern {
    /// Tokens: `{app}` `{title}` `{kind}` `{seq}` `{date}` and `{app|kind}`
    /// (app if present, else kind). Never a bare `pix`/`vix`.
    pub name_template: String,
    pub destination: Destination,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Default for LandingPattern {
    fn default() -> Self {
        Self {
            name_template: "{app|kind}-{seq}".into(),
            destination: Destination::LibraryBucket {
                bucket: "scratch".into(),
            },
            tags: Vec::new(),
        }
    }
}

impl LandingPattern {
    /// Expand the template with a vehicle's context into a kebabed, clamped,
    /// filesystem-safe stem — never empty, never a bare `pix` (falls back to
    /// `<kind>-<seq>` if the template yields nothing).
    pub fn expand_name(&self, cx: &NameContext) -> String {
        let app_or_kind = cx.app.filter(|s| !s.is_empty()).unwrap_or(cx.kind);
        let mut out = self.name_template.clone();
        for (token, value) in [
            ("{app|kind}", app_or_kind),
            ("{app}", cx.app.unwrap_or("")),
            ("{title}", cx.title.unwrap_or("")),
            ("{kind}", cx.kind),
            ("{date}", cx.date),
        ] {
            out = out.replace(token, value);
        }
        out = out.replace("{seq}", &cx.seq.to_string());
        let stem = kebab(&out);
        if stem.is_empty() {
            kebab(&format!("{}-{}", cx.kind, cx.seq))
        } else {
            stem
        }
    }
}

/// The policy: mode + pattern. Persisted operator STATE (Ring 0 `.userdata/`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LandingPolicy {
    #[serde(default)]
    pub mode: LandingMode,
    #[serde(default)]
    pub pattern: LandingPattern,
}

const FILE: &str = "landing-policy.json";

impl LandingPolicy {
    /// Load from `<userdata>/landing-policy.json`, or the default if absent or
    /// unreadable — a corrupt policy never blocks a capture.
    pub fn load(userdata: &Path) -> Self {
        std::fs::read_to_string(userdata.join(FILE))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Save to `<userdata>/landing-policy.json` (crash-safe tmp+rename).
    pub fn save(&self, userdata: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(userdata)?;
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let tmp = userdata.join(format!("{FILE}.tmp"));
        std::fs::write(&tmp, json)?;
        std::fs::rename(tmp, userdata.join(FILE))
    }

    /// Resolve the policy for a kind — GLOBAL today (the growth seam to
    /// per-kind patterns later; charter §3). The keyed signature is
    /// intentional: widening it is additive, not a rewrite.
    pub fn for_kind(&self, _kind: CaptureKind) -> &LandingPolicy {
        self
    }

    /// The organized inbox-relative directory for a landing —
    /// `<bucket>/<kind>` for a library destination (the caller joins the
    /// expanded name + extension). `None` for a non-library destination.
    pub fn inbox_subdir(&self, kind: CaptureKind) -> Option<String> {
        self.pattern
            .destination
            .bucket()
            .map(|b| format!("{}/{}", kebab(b), kind.as_str()))
    }
}

/// Lowercase kebab-case: alphanumeric runs joined by `-`, other chars are
/// separators, trimmed, clamped to 60 chars. The one filesystem-name law.
pub fn kebab(s: &str) -> String {
    let mut out = String::new();
    let mut pending_dash = false;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            if pending_dash && !out.is_empty() {
                out.push('-');
            }
            pending_dash = false;
            out.push(c.to_ascii_lowercase());
            if out.len() >= 60 {
                break;
            }
        } else {
            pending_dash = true;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kebab_is_filesystem_safe() {
        assert_eq!(kebab("Foo Bar!!"), "foo-bar");
        assert_eq!(kebab("  spaced  out  "), "spaced-out");
        assert_eq!(kebab("already-kebab"), "already-kebab");
        assert_eq!(kebab(""), "");
        assert_eq!(kebab("!!!"), "");
        assert_eq!(kebab(&"x".repeat(200)).len(), 60, "clamped");
    }

    #[test]
    fn expand_name_never_bare_and_uses_context() {
        let p = LandingPattern::default(); // "{app|kind}-{seq}"
        let img = NameContext {
            kind: "image",
            seq: 3,
            ..Default::default()
        };
        assert_eq!(p.expand_name(&img), "image-3", "no app → kind");
        let with_app = NameContext {
            app: Some("Notepad"),
            kind: "image",
            seq: 7,
            ..Default::default()
        };
        assert_eq!(
            p.expand_name(&with_app),
            "notepad-7",
            "app wins in {{app|kind}}"
        );
        // An empty template still yields a meaningful name, never bare.
        let empty = LandingPattern {
            name_template: "".into(),
            ..LandingPattern::default()
        };
        assert_eq!(empty.expand_name(&img), "image-3", "fallback, never empty");
    }

    #[test]
    fn inbox_subdir_is_bucket_then_kind() {
        let mut p = LandingPolicy::default();
        p.pattern.destination = Destination::LibraryBucket {
            bucket: "My Shoot".into(),
        };
        assert_eq!(
            p.inbox_subdir(CaptureKind::Video).as_deref(),
            Some("my-shoot/video")
        );
        // A BUILD destination has no inbox bucket.
        p.pattern.destination = Destination::Episode {
            show: "s".into(),
            episode: "e".into(),
            chapter: "ch01".into(),
            slot: None,
        };
        assert_eq!(p.inbox_subdir(CaptureKind::Video), None);
    }

    #[test]
    fn policy_round_trips_through_userdata() {
        let dir = std::env::temp_dir().join(format!("dpl-landing-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        // Absent → default (never blocks capture).
        assert_eq!(LandingPolicy::load(&dir), LandingPolicy::default());
        let mut p = LandingPolicy {
            mode: LandingMode::Ask,
            ..Default::default()
        };
        p.pattern.tags = vec!["demo".into()];
        p.save(&dir).unwrap();
        assert_eq!(LandingPolicy::load(&dir), p, "round-trips");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn capture_kinds_carry_stable_strings() {
        assert_eq!(CaptureKind::Image.as_str(), "image");
        assert_eq!(CaptureKind::Guide.as_str(), "guide");
    }
}
