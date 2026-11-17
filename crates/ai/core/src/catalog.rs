//! The AI catalog: every AI DevPULSE knows, as data (AH1a, chat plan §4.8).
//!
//! Two files, both compiled into this crate and never read from disk or the
//! network:
//!
//! - `catalog/acp-registry.json` is a pinned copy of the Agent Client Protocol
//!   registry (about fifty coding agents, with how each one is launched). Only
//!   `cargo xtask ai-catalog refresh` replaces it, and the PR that does is the
//!   review.
//! - `catalog/devpulse.json` holds only what DevPULSE adds, keyed by the same
//!   ids: how each AI's own tool answers "are you signed in", where it installs,
//!   how its running process is recognised, its look, and which adapter (if
//!   any) chats with it. It also lists the desktop apps the registry does not.
//!
//! An overlay entry that names a registry entry (`acp`) absorbs it, so one AI
//! is listed once. Every other registry entry is listed under its own id.

use crate::capability::Capability;
use crate::presence::PresenceSpec;
use crate::provider::{
    ProviderAccent, ProviderDescriptor, ProviderId, ProviderManifest, ProviderProfileId,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

/// The vendored ACP registry, byte-for-byte as it was pinned.
pub const ACP_REGISTRY: &str = include_str!("../catalog/acp-registry.json");
/// DevPULSE's own entries.
pub const DEVPULSE_ENTRIES: &str = include_str!("../catalog/devpulse.json");

/// An operating system a tell or install location is written for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Os {
    Windows,
    Linux,
    Macos,
}

impl Os {
    /// The OS this build runs on, when the catalog has entries for it.
    pub const fn current() -> Option<Self> {
        if cfg!(windows) {
            Some(Self::Windows)
        } else if cfg!(target_os = "linux") {
            Some(Self::Linux)
        } else if cfg!(target_os = "macos") {
            Some(Self::Macos)
        } else {
            None
        }
    }
}

/// What kind of thing an entry is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryKind {
    /// A coding agent with a CLI DevPULSE can (or one day will) chat through.
    Agent,
    /// A desktop app. DevPULSE detects it and offers to open it, nothing
    /// more: desktop apps expose no local chat interface (R12).
    Desktop,
}

/// How a running process is recognised as one AI.
///
/// The process name alone is not enough: Claude Desktop and the Claude Code
/// CLI are both `claude.exe`. A tell matches the name and, when it lists any,
/// one of the path fragments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tell {
    pub os: Os,
    /// The executable's file name, matched without regard to case.
    pub process: &'static str,
    /// Lowercase fragments of the executable's path with `/` separators. Empty
    /// means any path.
    pub path_contains: &'static [&'static str],
    /// Seen on a real machine, rather than authored from vendor documentation.
    pub verified: bool,
}

impl Tell {
    /// Whether a running process is this AI. `exe` may use either separator
    /// and any case.
    pub fn matches(&self, os: Os, process: &str, exe: &str) -> bool {
        if self.os != os || !process.eq_ignore_ascii_case(self.process) {
            return false;
        }
        if self.path_contains.is_empty() {
            return true;
        }
        let path = exe.replace('\\', "/").to_ascii_lowercase();
        self.path_contains
            .iter()
            .any(|fragment| path.contains(fragment))
    }
}

/// A place an AI installs itself. `%VAR%` (Windows) and a leading `~/` are
/// expanded by the host; existence is all that is ever checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstallPath {
    pub os: Os,
    pub path: &'static str,
    pub verified: bool,
}

/// What the ACP registry says about an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcpListing {
    pub id: &'static str,
    pub version: &'static str,
    pub description: &'static str,
    pub repository: Option<&'static str>,
    pub website: Option<&'static str>,
    pub license: Option<&'static str>,
    /// `npx <package>` launches it, when the registry says so.
    pub npx: Option<&'static str>,
    /// `uvx <package>` launches it, when the registry says so.
    pub uvx: Option<&'static str>,
    /// Platform targets with a downloadable binary (`windows-x86_64`, …).
    pub binary_targets: &'static [&'static str],
}

/// One AI the catalog knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CatalogEntry {
    pub manifest: ProviderManifest,
    pub kind: EntryKind,
    /// The DevPULSE adapter that chats with it, once one is qualified.
    pub adapter: Option<&'static str>,
    /// Where the operator can read about or get it.
    pub homepage: Option<&'static str>,
    /// What was verified about this entry, and how. Shown to developers; it is
    /// how an unverified sign-in check stays labelled as one.
    pub note: Option<&'static str>,
    /// How its own tool answers "are you signed in".
    pub presence: Option<PresenceSpec>,
    pub acp: Option<AcpListing>,
    pub installs: &'static [InstallPath],
    pub tells: &'static [Tell],
}

impl CatalogEntry {
    pub const fn id(&self) -> ProviderId {
        self.manifest.descriptor.id
    }
}

/// Which registry copy is vendored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegistryPin {
    pub source: &'static str,
    pub fetched: &'static str,
    pub sha256: &'static str,
}

/// Every AI DevPULSE knows, in display order: DevPULSE's own entries in file
/// order, then the remaining registry agents by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Catalog {
    pub pin: RegistryPin,
    pub entries: Vec<CatalogEntry>,
}

impl Catalog {
    pub fn get(&self, id: ProviderId) -> Option<&CatalogEntry> {
        self.entries.iter().find(|entry| entry.id() == id)
    }

    /// The manifest of a catalog AI.
    pub fn manifest(&self, id: ProviderId) -> Option<ProviderManifest> {
        self.get(id).map(|entry| entry.manifest)
    }

    /// Every AI whose own tool can say whether the operator is signed in.
    pub fn presence_specs(&self) -> Vec<(ProviderId, PresenceSpec)> {
        self.entries
            .iter()
            .filter_map(|entry| entry.presence.map(|spec| (entry.id(), spec)))
            .collect()
    }
}

/// The catalog compiled into this build. Both files are proven to parse by
/// this module's tests, which `cargo green` runs on every PR.
pub fn catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| match parse(ACP_REGISTRY, DEVPULSE_ENTRIES) {
        Ok(catalog) => catalog,
        Err(error) => panic!("the compiled-in AI catalog is invalid: {error}"),
    })
}

/// A catalog AI's manifest, for code that names one of DevPULSE's own AIs by
/// its provider id. `None` only for an id the catalog does not have.
pub fn manifest(id: ProviderId) -> Option<ProviderManifest> {
    catalog().manifest(id)
}

/// Parse and merge a registry and an overlay. Public so tests can prove the
/// merge rules on small inputs.
pub fn parse(registry: &str, overlay: &str) -> Result<Catalog, String> {
    let registry: RawRegistry =
        serde_json::from_str(registry).map_err(|error| format!("ACP registry: {error}"))?;
    let overlay: RawOverlay =
        serde_json::from_str(overlay).map_err(|error| format!("DevPULSE entries: {error}"))?;
    if overlay.schema != "dpl.ai-catalog/1" {
        return Err(format!("unknown catalog schema {:?}", overlay.schema));
    }
    let mut agents: BTreeMap<String, RawAgent> = BTreeMap::new();
    for agent in registry.agents {
        let id = agent.id.clone();
        if agents.insert(id.clone(), agent).is_some() {
            return Err(format!("the ACP registry lists {id} twice"));
        }
    }

    let mut entries = Vec::new();
    let mut ids = BTreeSet::new();
    let mut claimed = BTreeSet::new();
    for raw in overlay.entries {
        if !ids.insert(raw.id.clone()) {
            return Err(format!("{} is listed twice", raw.id));
        }
        let acp = match &raw.acp {
            Some(acp_id) => {
                let agent = agents.get(acp_id).ok_or_else(|| {
                    format!("{} names ACP entry {acp_id}, which is missing", raw.id)
                })?;
                if !claimed.insert(acp_id.clone()) {
                    return Err(format!("ACP entry {acp_id} is claimed twice"));
                }
                Some(listing(agent))
            }
            None => None,
        };
        entries.push(overlay_entry(raw, acp)?);
    }

    let mut rest: Vec<&RawAgent> = agents
        .values()
        .filter(|agent| !claimed.contains(&agent.id))
        .collect();
    rest.sort_by_key(|agent| agent.name.to_lowercase());
    for (index, agent) in rest.into_iter().enumerate() {
        if !ids.insert(agent.id.clone()) {
            return Err(format!(
                "ACP entry {} has the id of a DevPULSE entry that does not claim it",
                agent.id
            ));
        }
        entries.push(registry_entry(agent, index));
    }

    Ok(Catalog {
        pin: RegistryPin {
            source: leak(overlay.acp_registry.source),
            fetched: leak(overlay.acp_registry.fetched),
            sha256: leak(overlay.acp_registry.sha256),
        },
        entries,
    })
}

fn overlay_entry(raw: RawEntry, acp: Option<AcpListing>) -> Result<CatalogEntry, String> {
    let id = leak(raw.id);
    if raw.kind == EntryKind::Desktop && (raw.adapter.is_some() || raw.presence.is_some()) {
        return Err(format!(
            "{id} is a desktop app; it cannot name an adapter or a sign-in command"
        ));
    }
    let capabilities = raw
        .capabilities
        .iter()
        .map(|slug| {
            Capability::from_slug(slug).ok_or_else(|| format!("{id}: unknown capability {slug}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    for tell in &raw.tells {
        if tell.process.is_empty()
            || tell
                .path_contains
                .iter()
                .any(|fragment| fragment.contains('\\') || *fragment != fragment.to_lowercase())
        {
            return Err(format!(
                "{id}: a tell needs a process name and lowercase `/` path fragments"
            ));
        }
    }
    let presence = match raw.presence {
        Some(spec) => {
            let lowercase = |needles: &[String]| {
                needles
                    .iter()
                    .all(|needle| *needle == needle.to_ascii_lowercase())
            };
            if spec.program.is_empty()
                || spec.signed_in.is_empty()
                || !lowercase(&spec.signed_in)
                || !lowercase(&spec.signed_out)
            {
                return Err(format!(
                    "{id}: presence needs a program and lowercase signed-in words"
                ));
            }
            Some(PresenceSpec {
                program: leak(spec.program),
                args: leak_all(spec.args),
                signed_in: leak_all(spec.signed_in),
                signed_out: leak_all(spec.signed_out),
                login: leak_all(spec.login),
            })
        }
        None => None,
    };
    let summary = raw
        .summary
        .or_else(|| acp.map(|listing| listing.description.to_owned()))
        .unwrap_or_default();
    Ok(CatalogEntry {
        manifest: ProviderManifest {
            descriptor: ProviderDescriptor {
                id: ProviderId(id),
                label: leak(raw.label),
                short_label: leak(raw.short_label),
                glyph: leak(raw.glyph),
                accent: raw.accent.into(),
                summary: leak(summary),
                since: leak(raw.since.unwrap_or_default()),
            },
            profile: ProviderProfileId(leak(
                raw.profile.unwrap_or_else(|| format!("{id}-default")),
            )),
            target_capabilities: Box::leak(capabilities.into_boxed_slice()),
            delivery_slice: leak(raw.delivery_slice.unwrap_or_else(|| "A2b".to_owned())),
            evidence: leak(
                raw.evidence
                    .or_else(|| raw.homepage.clone())
                    .unwrap_or_default(),
            ),
        },
        kind: raw.kind,
        adapter: raw.adapter.map(leak),
        homepage: raw.homepage.map(leak),
        note: raw.note.map(leak),
        presence,
        acp,
        installs: Box::leak(
            raw.installs
                .into_iter()
                .map(|install| InstallPath {
                    os: install.os,
                    path: leak(install.path),
                    verified: install.verified,
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
        tells: Box::leak(
            raw.tells
                .into_iter()
                .map(|tell| Tell {
                    os: tell.os,
                    process: leak(tell.process),
                    path_contains: leak_all(tell.path_contains),
                    verified: tell.verified,
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
    })
}

/// A registry agent DevPULSE adds nothing to yet: listed by its own words,
/// with a look derived from its name so the list stays stable.
fn registry_entry(agent: &RawAgent, index: usize) -> CatalogEntry {
    const ACCENTS: [ProviderAccent; 5] = [
        ProviderAccent::Cyan,
        ProviderAccent::Violet,
        ProviderAccent::Amber,
        ProviderAccent::Lime,
        ProviderAccent::Magenta,
    ];
    let listing = listing(agent);
    let glyph = agent
        .name
        .chars()
        .find(|c| c.is_alphanumeric())
        .map(|c| c.to_uppercase().collect::<String>())
        .unwrap_or_else(|| "?".to_owned());
    let homepage = listing.website.or(listing.repository);
    CatalogEntry {
        manifest: ProviderManifest {
            descriptor: ProviderDescriptor {
                id: ProviderId(listing.id),
                label: leak(agent.name.clone()),
                short_label: leak(agent.name.clone()),
                glyph: leak(glyph),
                accent: ACCENTS[index % ACCENTS.len()],
                summary: listing.description,
                since: "",
            },
            profile: ProviderProfileId(leak(format!("{}-default", agent.id))),
            target_capabilities: &[],
            delivery_slice: "A2b",
            evidence: homepage.unwrap_or(""),
        },
        kind: EntryKind::Agent,
        adapter: None,
        homepage,
        note: None,
        presence: None,
        acp: Some(listing),
        installs: &[],
        tells: &[],
    }
}

fn listing(agent: &RawAgent) -> AcpListing {
    AcpListing {
        id: leak(agent.id.clone()),
        version: leak(agent.version.clone()),
        description: leak(agent.description.clone().unwrap_or_default()),
        repository: agent.repository.clone().map(leak),
        website: agent.website.clone().map(leak),
        license: agent.license.clone().map(leak),
        npx: agent
            .distribution
            .npx
            .as_ref()
            .map(|npx| leak(npx.package.clone())),
        uvx: agent
            .distribution
            .uvx
            .as_ref()
            .map(|uvx| leak(uvx.package.clone())),
        binary_targets: leak_all(agent.distribution.binary.keys().cloned().collect()),
    }
}

/// The catalog lives for the whole process, so its strings are made
/// `'static` once, at load, and the manifest types keep their borrowed form.
fn leak(value: String) -> &'static str {
    Box::leak(value.into_boxed_str())
}

fn leak_all(values: Vec<String>) -> &'static [&'static str] {
    Box::leak(
        values
            .into_iter()
            .map(leak)
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    )
}

#[derive(Deserialize)]
struct RawRegistry {
    agents: Vec<RawAgent>,
}

#[derive(Deserialize)]
struct RawAgent {
    id: String,
    name: String,
    version: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    repository: Option<String>,
    #[serde(default)]
    website: Option<String>,
    #[serde(default)]
    license: Option<String>,
    #[serde(default)]
    distribution: RawDistribution,
}

#[derive(Deserialize, Default)]
struct RawDistribution {
    #[serde(default)]
    binary: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    npx: Option<RawPackage>,
    #[serde(default)]
    uvx: Option<RawPackage>,
}

#[derive(Deserialize)]
struct RawPackage {
    package: String,
}

#[derive(Deserialize)]
struct RawOverlay {
    schema: String,
    acp_registry: RawPin,
    entries: Vec<RawEntry>,
}

#[derive(Deserialize)]
struct RawPin {
    source: String,
    fetched: String,
    sha256: String,
}

#[derive(Deserialize)]
struct RawEntry {
    id: String,
    #[serde(default)]
    acp: Option<String>,
    kind: EntryKind,
    label: String,
    short_label: String,
    glyph: String,
    accent: RawAccent,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    since: Option<String>,
    #[serde(default)]
    profile: Option<String>,
    #[serde(default)]
    capabilities: Vec<String>,
    #[serde(default)]
    delivery_slice: Option<String>,
    #[serde(default)]
    evidence: Option<String>,
    #[serde(default)]
    adapter: Option<String>,
    #[serde(default)]
    homepage: Option<String>,
    #[serde(default)]
    note: Option<String>,
    #[serde(default)]
    presence: Option<RawPresence>,
    #[serde(default)]
    installs: Vec<RawInstall>,
    #[serde(default)]
    tells: Vec<RawTell>,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum RawAccent {
    Amber,
    Cyan,
    Violet,
    Magenta,
    Lime,
}

impl From<RawAccent> for ProviderAccent {
    fn from(accent: RawAccent) -> Self {
        match accent {
            RawAccent::Amber => Self::Amber,
            RawAccent::Cyan => Self::Cyan,
            RawAccent::Violet => Self::Violet,
            RawAccent::Magenta => Self::Magenta,
            RawAccent::Lime => Self::Lime,
        }
    }
}

#[derive(Deserialize)]
struct RawPresence {
    program: String,
    #[serde(default)]
    args: Vec<String>,
    signed_in: Vec<String>,
    #[serde(default)]
    signed_out: Vec<String>,
    #[serde(default)]
    login: Vec<String>,
}

#[derive(Deserialize)]
struct RawInstall {
    os: Os,
    path: String,
    #[serde(default)]
    verified: bool,
}

#[derive(Deserialize)]
struct RawTell {
    os: Os,
    process: String,
    #[serde(default)]
    path_contains: Vec<String>,
    #[serde(default)]
    verified: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    const REGISTRY: &str = r#"{"version":"1.0.0","agents":[
        {"id":"alpha-acp","name":"Alpha","version":"1.0.0","description":"Alpha agent",
         "distribution":{"npx":{"package":"@a/alpha","args":["--acp"]}}},
        {"id":"beta","name":"beta tool","version":"2.0.0",
         "distribution":{"binary":{"windows-x86_64":{"archive":"https://x","cmd":"./b"}}}}
    ]}"#;

    fn overlay(entries: &str) -> String {
        format!(
            r#"{{"schema":"dpl.ai-catalog/1",
                "acp_registry":{{"file":"r.json","source":"s","fetched":"2026-11-16","sha256":"00"}},
                "entries":[{entries}]}}"#
        )
    }

    const ALPHA: &str = r#"{"id":"alpha","acp":"alpha-acp","kind":"agent","label":"Alpha AI",
        "short_label":"Alpha","glyph":"A","accent":"amber","capabilities":["stream-turns"],
        "adapter":"alpha","presence":{"program":"alpha","args":["status"],
        "signed_in":["logged in"],"signed_out":["not logged in"],"login":["login"]}}"#;

    #[test]
    fn the_vendored_registry_is_the_copy_that_was_pinned() {
        let digest = Sha256::digest(ACP_REGISTRY.as_bytes());
        let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
        assert_eq!(
            hex,
            catalog().pin.sha256,
            "re-run `cargo xtask ai-catalog refresh`"
        );
    }

    #[test]
    fn the_compiled_in_catalog_parses_and_keeps_devpulses_ais_first() {
        let catalog = catalog();
        let first: Vec<&str> = catalog.entries[..5].iter().map(|e| e.id().0).collect();
        assert_eq!(first, ["codex", "claude", "cursor", "gemini", "grok"]);
        assert!(
            catalog.entries.len() > 40,
            "the registry's agents are listed too"
        );
        // Claimed registry entries are not listed a second time.
        for claimed in ["codex-acp", "claude-acp", "grok-build"] {
            assert!(catalog.get(ProviderId(claimed)).is_none(), "{claimed}");
        }
        assert_eq!(
            catalog
                .get(ProviderId("grok"))
                .and_then(|e| e.acp)
                .map(|a| a.id),
            Some("grok-build")
        );
    }

    #[test]
    fn only_codex_and_grok_have_a_qualified_adapter() {
        let adapters: Vec<_> = catalog()
            .entries
            .iter()
            .filter_map(|entry| entry.adapter.map(|adapter| (entry.id().0, adapter)))
            .collect();
        assert_eq!(adapters, [("codex", "codex"), ("grok", "grok")]);
    }

    #[test]
    fn every_ai_with_a_sign_in_check_is_one_of_devpulses_five() {
        let ids: Vec<_> = catalog()
            .presence_specs()
            .into_iter()
            .map(|(id, _)| id.0)
            .collect();
        assert_eq!(ids, ["codex", "claude", "cursor", "gemini", "grok"]);
    }

    #[test]
    fn claude_desktop_and_the_claude_cli_are_told_apart_by_path() {
        let desktop = catalog().get(ProviderId("claude-desktop")).unwrap();
        let cli = catalog().get(ProviderId("claude")).unwrap();
        // The two real paths read off this laptop on 2026-11-16.
        let app =
            r"C:\Program Files\WindowsApps\Claude_5.11492.5.0_x64__pzs8sxrjxfjjc\app\Claude.exe";
        let code = r"c:\Users\me\.vscode\extensions\anthropic.claude-code-2.1.323-win32-x64\resources\native-binary\claude.exe";
        let hit = |entry: &CatalogEntry, path: &str| {
            entry
                .tells
                .iter()
                .any(|tell| tell.matches(Os::Windows, "claude.exe", path))
        };
        assert!(hit(desktop, app) && !hit(desktop, code));
        assert!(hit(cli, code) && !hit(cli, app));
    }

    #[test]
    fn a_tell_never_matches_another_os_or_name() {
        let tell = Tell {
            os: Os::Windows,
            process: "grok bot.exe",
            path_contains: &[],
            verified: true,
        };
        assert!(tell.matches(
            Os::Windows,
            "Grok Bot.exe",
            r"C:\Program Files\Grok Bot\Grok Bot.exe"
        ));
        assert!(!tell.matches(Os::Linux, "Grok Bot.exe", "/opt/grok"));
        assert!(!tell.matches(Os::Windows, "grok.exe", r"C:\x\grok.exe"));
    }

    #[test]
    fn desktop_apps_have_no_adapter_or_sign_in_check() {
        for entry in &catalog().entries {
            if entry.kind == EntryKind::Desktop {
                assert!(
                    entry.adapter.is_none() && entry.presence.is_none(),
                    "{}",
                    entry.id()
                );
            }
        }
    }

    #[test]
    fn a_claimed_entry_merges_and_the_rest_are_listed_by_name() {
        let catalog = parse(REGISTRY, &overlay(ALPHA)).unwrap();
        let ids: Vec<_> = catalog.entries.iter().map(|e| e.id().0).collect();
        assert_eq!(ids, ["alpha", "beta"]);
        let alpha = &catalog.entries[0];
        assert_eq!(alpha.manifest.descriptor.summary, "Alpha agent");
        assert_eq!(alpha.acp.unwrap().npx, Some("@a/alpha"));
        assert_eq!(
            alpha.manifest.target_capabilities,
            [Capability::StreamTurns]
        );
        let beta = &catalog.entries[1];
        assert_eq!(beta.manifest.descriptor.glyph, "B");
        assert_eq!(beta.acp.unwrap().binary_targets, ["windows-x86_64"]);
        assert!(beta.presence.is_none() && beta.adapter.is_none());
    }

    #[test]
    fn broken_overlays_are_refused_by_name() {
        let missing = ALPHA.replace("alpha-acp", "nobody");
        assert!(parse(REGISTRY, &overlay(&missing))
            .unwrap_err()
            .contains("missing"));
        let twice = format!("{ALPHA},{ALPHA}");
        assert!(parse(REGISTRY, &overlay(&twice))
            .unwrap_err()
            .contains("twice"));
        let capability = ALPHA.replace("stream-turns", "telepathy");
        assert!(parse(REGISTRY, &overlay(&capability))
            .unwrap_err()
            .contains("telepathy"));
        let upper = ALPHA.replace("\"logged in\"", "\"Logged In\"");
        assert!(parse(REGISTRY, &overlay(&upper))
            .unwrap_err()
            .contains("lowercase"));
        let desktop = ALPHA.replace("\"agent\"", "\"desktop\"");
        assert!(parse(REGISTRY, &overlay(&desktop))
            .unwrap_err()
            .contains("desktop app"));
        let clash = r#"{"id":"beta","kind":"desktop","label":"B","short_label":"B","glyph":"B","accent":"lime"}"#;
        assert!(parse(REGISTRY, &overlay(clash))
            .unwrap_err()
            .contains("does not claim"));
    }
}
