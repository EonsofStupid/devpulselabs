//! Rooms and roles as data (R284): `dpl.role/1` and `dpl.room/1`.
//!
//! The schemas are `schemas/role.v1.schema.json` and
//! `schemas/room.v1.schema.json`. The built-in library is the JSON under
//! `library/`, compiled in. The research every entry cites is
//! `docs/research/chat-rooms-and-roles.md`. A new role is a new file: the
//! operator's own go in their library folder, and the app hands their text
//! here. This module never reads disk.
//!
//! The rules a schema cannot say live in [`validate_role`] and
//! [`validate_room`]: a seat names a real role; every parameter a role uses
//! is declared and filled; a room's selection seat is a Selector; and a
//! Selector must forbid merging (select, never blend; R25).

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const ROLE_SCHEMA: &str = "dpl.role/1";
pub const ROOM_SCHEMA: &str = "dpl.room/1";

/// The built-in roles, as authored.
pub const BUILTIN_ROLES: &[(&str, &str)] = &[
    ("advocate", include_str!("../library/roles/advocate.json")),
    (
        "domain-expert",
        include_str!("../library/roles/domain-expert.json"),
    ),
    ("dreamer", include_str!("../library/roles/dreamer.json")),
    (
        "fact-checker",
        include_str!("../library/roles/fact-checker.json"),
    ),
    ("planner", include_str!("../library/roles/planner.json")),
    (
        "pre-mortem",
        include_str!("../library/roles/pre-mortem.json"),
    ),
    ("realist", include_str!("../library/roles/realist.json")),
    ("red-team", include_str!("../library/roles/red-team.json")),
    ("selector", include_str!("../library/roles/selector.json")),
    ("skeptic", include_str!("../library/roles/skeptic.json")),
    ("steelman", include_str!("../library/roles/steelman.json")),
];

/// The built-in rooms, as authored.
pub const BUILTIN_ROOMS: &[(&str, &str)] = &[
    (
        "author-critic",
        include_str!("../library/rooms/author-critic.json"),
    ),
    (
        "brainstorm",
        include_str!("../library/rooms/brainstorm.json"),
    ),
    ("council", include_str!("../library/rooms/council.json")),
    ("debate", include_str!("../library/rooms/debate.json")),
    ("router", include_str!("../library/rooms/router.json")),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Stance {
    Generate,
    Ground,
    Critique,
    Adversarial,
    Plan,
    Select,
}

impl Stance {
    /// The schema's name for it (`critique`, `generate`…).
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Generate => "generate",
            Self::Ground => "ground",
            Self::Critique => "critique",
            Self::Adversarial => "adversarial",
            Self::Plan => "plan",
            Self::Select => "select",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Generate => "Generates",
            Self::Ground => "Grounds",
            Self::Critique => "Critiques",
            Self::Adversarial => "Attacks",
            Self::Plan => "Plans",
            Self::Select => "Selects",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    None,
    Anonymized,
    Full,
}

impl Visibility {
    /// What a seat sees of the others, in plain words.
    pub const fn label(self) -> &'static str {
        match self {
            Self::None => "sees no other answers",
            Self::Anonymized => "sees others' answers without names",
            Self::Full => "sees everything said",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Evidence {
    Strong,
    Moderate,
    Practice,
}

impl Evidence {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Strong => "Strong evidence",
            Self::Moderate => "Moderate evidence",
            Self::Practice => "Long practice",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Effort {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Method {
    pub name: String,
    pub steps: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RolePermissions {
    pub talk_only: bool,
    pub tools_allowed: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelPrefs {
    #[serde(default)]
    pub preferred_vendors: Vec<String>,
    pub avoid_same_vendor_as: Vec<String>,
    #[serde(default)]
    pub effort: Option<Effort>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Budget {
    pub max_turns: u32,
    #[serde(default)]
    pub max_output_tokens: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Parameter {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub sources: Vec<String>,
    pub evidence: Evidence,
    #[serde(default)]
    pub note: Option<String>,
}

/// One role a participant plays (`dpl.role/1`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoleCard {
    pub schema: String,
    pub id: String,
    pub version: String,
    pub name: String,
    pub summary: String,
    pub purpose: String,
    pub stance: Stance,
    pub method: Method,
    pub instructions: String,
    pub must_not: Vec<String>,
    /// A JSON Schema of what the role must produce.
    pub output_contract: serde_json::Value,
    pub rubric: Vec<String>,
    pub speaks_when: Vec<String>,
    pub visibility: Visibility,
    pub permissions: RolePermissions,
    pub model_prefs: ModelPrefs,
    pub budget: Budget,
    #[serde(default)]
    pub examples: Vec<String>,
    pub tags: Vec<String>,
    #[serde(default)]
    pub parameters: Vec<Parameter>,
    pub provenance: Provenance,
}

impl RoleCard {
    /// The fields the role's output contract requires, in order.
    pub fn output_fields(&self) -> Vec<&str> {
        self.output_contract
            .get("required")
            .and_then(serde_json::Value::as_array)
            .map(|fields| {
                fields
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RoomType {
    Council,
    Debate,
    AuthorCritic,
    Router,
    Brainstorm,
}

impl RoomType {
    /// The schema's name for it (`council`, `author-critic`…).
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Council => "council",
            Self::Debate => "debate",
            Self::AuthorCritic => "author-critic",
            Self::Router => "router",
            Self::Brainstorm => "brainstorm",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Council => "Council",
            Self::Debate => "Debate",
            Self::AuthorCritic => "Author + Critic",
            Self::Router => "Router",
            Self::Brainstorm => "Brainstorm",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Seat {
    pub seat: String,
    /// `role-id@major`.
    pub role: String,
    pub count: u32,
    #[serde(default)]
    pub parameters: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PhaseMode {
    Parallel,
    Sequential,
}

impl PhaseMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Parallel => "all at once",
            Self::Sequential => "one after another",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Phase {
    pub name: String,
    pub seats: Vec<String>,
    pub mode: PhaseMode,
    pub visibility: Visibility,
    pub max_turns: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TurnPolicy {
    Fixed,
    RoundRobin,
    Selector,
    Router,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StopRule {
    pub max_rounds: u32,
    #[serde(default)]
    pub budget_tokens: Option<u64>,
    #[serde(default)]
    pub stability: bool,
    #[serde(default)]
    pub pass_threshold: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SelectionMethod {
    Judge,
    Panel,
    Vote,
    Human,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub method: SelectionMethod,
    pub seat: String,
    pub blind: bool,
    pub swap_order: bool,
    pub exclude_author_vendors: bool,
    pub allow_none: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CarryForward {
    StructuredOutputs,
    FullTranscript,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RecordItem {
    Winner,
    RunnerUp,
    Rationale,
    Dissent,
    Flips,
}

/// A room: who sits in it, in what order they speak, and how one answer is
/// chosen (`dpl.room/1`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoomSpec {
    pub schema: String,
    pub id: String,
    pub version: String,
    pub name: String,
    #[serde(rename = "type")]
    pub room_type: RoomType,
    pub summary: String,
    pub seats: Vec<Seat>,
    pub phases: Vec<Phase>,
    pub turn_policy: TurnPolicy,
    pub stop: StopRule,
    #[serde(default)]
    pub selection: Option<Selection>,
    pub carry_forward: CarryForward,
    pub record: Vec<RecordItem>,
    pub provenance: Provenance,
}

/// One problem with a library file, at a path the operator can find.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryIssue {
    pub at: String,
    pub problem: String,
}

impl LibraryIssue {
    fn new(at: impl Into<String>, problem: impl Into<String>) -> Self {
        Self {
            at: at.into(),
            problem: problem.into(),
        }
    }
}

impl fmt::Display for LibraryIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.at, self.problem)
    }
}

fn is_kebab(value: &str) -> bool {
    let mut parts = value.split('-');
    let first_ok = parts.next().is_some_and(|first| {
        first
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_lowercase())
            && first
                .chars()
                .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
    });
    first_ok
        && parts.all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
        })
}

/// `1.2.3` → `(1, 2, 3)`.
pub fn semver(version: &str) -> Option<(u64, u64, u64)> {
    let mut parts = version.split('.');
    let parsed = (
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    );
    parts.next().is_none().then_some(parsed)
}

/// `skeptic@1` → `("skeptic", 1)`.
pub fn role_reference(reference: &str) -> Option<(&str, u64)> {
    let (id, major) = reference.split_once('@')?;
    Some((id, major.parse().ok()?))
}

/// The `{name}` placeholders a text uses.
fn placeholders(text: &str) -> BTreeSet<&str> {
    let mut found = BTreeSet::new();
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else { break };
        let name = &after[..close];
        if !name.is_empty()
            && name
                .chars()
                .all(|character| character.is_ascii_lowercase() || character == '_')
        {
            found.insert(name);
        }
        rest = &after[close + 1..];
    }
    found
}

/// The rules a role must keep beyond its schema.
pub fn validate_role(role: &RoleCard) -> Vec<LibraryIssue> {
    let at = |field: &str| format!("role {} · {field}", role.id);
    let mut issues = Vec::new();
    if role.schema != ROLE_SCHEMA {
        issues.push(LibraryIssue::new(
            at("schema"),
            format!("must be {ROLE_SCHEMA}"),
        ));
    }
    if !is_kebab(&role.id) {
        issues.push(LibraryIssue::new(at("id"), "must be kebab-case"));
    }
    if semver(&role.version).is_none() {
        issues.push(LibraryIssue::new(
            at("version"),
            "must be major.minor.patch",
        ));
    }
    if role.method.steps.is_empty() {
        issues.push(LibraryIssue::new(at("method"), "needs at least one step"));
    }
    if role.rubric.is_empty() {
        issues.push(LibraryIssue::new(
            at("rubric"),
            "needs at least one criterion",
        ));
    }
    if role.provenance.sources.is_empty() {
        issues.push(LibraryIssue::new(
            at("provenance"),
            "needs at least one source",
        ));
    }
    if role.permissions.talk_only && !role.permissions.tools_allowed.is_empty() {
        issues.push(LibraryIssue::new(
            at("permissions"),
            "a talk-only role cannot allow tools",
        ));
    }
    // Select, never blend (R25): a selecting role must forbid merging.
    if role.stance == Stance::Select
        && !role
            .must_not
            .iter()
            .any(|rule| rule.to_ascii_lowercase().contains("merge"))
    {
        issues.push(LibraryIssue::new(
            at("must_not"),
            "a selecting role must forbid merging candidates (select, never blend)",
        ));
    }
    // Every placeholder the role's text uses is declared, and every declared
    // parameter is used.
    let declared: BTreeSet<&str> = role.parameters.iter().map(|p| p.name.as_str()).collect();
    let mut used = placeholders(&role.instructions);
    for step in &role.method.steps {
        used.extend(placeholders(step));
    }
    for name in used.difference(&declared) {
        issues.push(LibraryIssue::new(
            at("parameters"),
            format!("{{{name}}} is used but not declared"),
        ));
    }
    for name in declared.difference(&used) {
        issues.push(LibraryIssue::new(
            at("parameters"),
            format!("{name} is declared but never used"),
        ));
    }
    issues
}

/// The rules a room must keep beyond its schema, checked against the roles
/// it can seat.
pub fn validate_room(room: &RoomSpec, roles: &[RoleCard]) -> Vec<LibraryIssue> {
    let at = |field: &str| format!("room {} · {field}", room.id);
    let mut issues = Vec::new();
    if room.schema != ROOM_SCHEMA {
        issues.push(LibraryIssue::new(
            at("schema"),
            format!("must be {ROOM_SCHEMA}"),
        ));
    }
    if !is_kebab(&room.id) {
        issues.push(LibraryIssue::new(at("id"), "must be kebab-case"));
    }
    if semver(&room.version).is_none() {
        issues.push(LibraryIssue::new(
            at("version"),
            "must be major.minor.patch",
        ));
    }
    let mut seats = BTreeMap::new();
    for seat in &room.seats {
        if seats.insert(seat.seat.as_str(), seat).is_some() {
            issues.push(LibraryIssue::new(
                at("seats"),
                format!("seat {} appears twice", seat.seat),
            ));
        }
        let Some(role) = resolve_role(roles, &seat.role) else {
            issues.push(LibraryIssue::new(
                at("seats"),
                format!(
                    "seat {} names {}, which is not in the library",
                    seat.seat, seat.role
                ),
            ));
            continue;
        };
        for parameter in &role.parameters {
            if !seat.parameters.contains_key(&parameter.name) {
                issues.push(LibraryIssue::new(
                    at("seats"),
                    format!(
                        "seat {} must fill {}'s {}",
                        seat.seat, role.id, parameter.name
                    ),
                ));
            }
        }
    }
    for phase in &room.phases {
        for seat in &phase.seats {
            if !seats.contains_key(seat.as_str()) {
                issues.push(LibraryIssue::new(
                    at("phases"),
                    format!(
                        "phase {} names seat {seat}, which the room does not have",
                        phase.name
                    ),
                ));
            }
        }
    }
    if let Some(selection) = &room.selection {
        match seats.get(selection.seat.as_str()) {
            None => issues.push(LibraryIssue::new(
                at("selection"),
                format!("the selection seat {} is not a seat", selection.seat),
            )),
            Some(seat) => {
                let selects = resolve_role(roles, &seat.role)
                    .is_some_and(|role| role.stance == Stance::Select);
                if !selects {
                    issues.push(LibraryIssue::new(
                        at("selection"),
                        format!(
                            "seat {} must be a selecting role (select, never blend)",
                            selection.seat
                        ),
                    ));
                }
            }
        }
    }
    issues
}

/// The role a `role-id@major` reference names: the highest version with
/// that major.
pub fn resolve_role<'a>(roles: &'a [RoleCard], reference: &str) -> Option<&'a RoleCard> {
    let (id, major) = role_reference(reference)?;
    roles
        .iter()
        .filter(|role| role.id == id && semver(&role.version).is_some_and(|v| v.0 == major))
        .max_by_key(|role| semver(&role.version))
}

/// Parse and check one role file's text.
pub fn parse_role(at: &str, text: &str) -> Result<RoleCard, Vec<LibraryIssue>> {
    let role: RoleCard = serde_json::from_str(text)
        .map_err(|error| vec![LibraryIssue::new(at, error.to_string())])?;
    let issues = validate_role(&role);
    if issues.is_empty() {
        Ok(role)
    } else {
        Err(issues)
    }
}

/// Parse one room file's text (checked later, against the whole library).
pub fn parse_room(at: &str, text: &str) -> Result<RoomSpec, Vec<LibraryIssue>> {
    serde_json::from_str(text).map_err(|error| vec![LibraryIssue::new(at, error.to_string())])
}

/// Everything the operator can seat and open: the built-ins plus their own
/// files, with every file that did not pass named.
#[derive(Debug, Clone, Default)]
pub struct Library {
    pub roles: Vec<RoleCard>,
    pub rooms: Vec<RoomSpec>,
    /// A file that was not loaded, and why.
    pub rejected: Vec<(String, Vec<LibraryIssue>)>,
}

impl Library {
    /// The built-in library alone.
    pub fn builtin() -> Self {
        Self::with_user(&[], &[])
    }

    /// The built-ins plus the operator's own role and room files, given as
    /// `(file name, text)`. A user entry with a built-in's id replaces it
    /// only with a higher version; otherwise it is rejected, named. A broken
    /// file is never loaded half-way.
    pub fn with_user(user_roles: &[(String, String)], user_rooms: &[(String, String)]) -> Self {
        let mut library = Self::default();
        let add_role =
            |library: &mut Self, at: &str, text: &str, builtin: bool| match parse_role(at, text) {
                Ok(role) => {
                    if let Some(existing) = library.roles.iter().position(|r| r.id == role.id) {
                        let newer =
                            semver(&role.version) > semver(&library.roles[existing].version);
                        if newer {
                            library.roles[existing] = role;
                        } else {
                            library.rejected.push((
                                at.to_owned(),
                                vec![LibraryIssue::new(
                                    at,
                                    format!(
                                        "{} {} does not replace version {}; raise its version",
                                        role.id, role.version, library.roles[existing].version
                                    ),
                                )],
                            ));
                        }
                    } else {
                        library.roles.push(role);
                    }
                }
                Err(issues) => {
                    let at = if builtin {
                        format!("built-in {at}")
                    } else {
                        at.to_owned()
                    };
                    library.rejected.push((at, issues));
                }
            };
        for (id, text) in BUILTIN_ROLES {
            add_role(&mut library, &format!("{id}.json"), text, true);
        }
        for (file, text) in user_roles {
            add_role(&mut library, file, text, false);
        }
        let room_texts = BUILTIN_ROOMS
            .iter()
            .map(|(id, text)| (format!("{id}.json"), (*text).to_owned()))
            .chain(user_rooms.iter().cloned());
        for (at, text) in room_texts {
            match parse_room(&at, &text) {
                Ok(room) => {
                    let issues = validate_room(&room, &library.roles);
                    if !issues.is_empty() {
                        library.rejected.push((at, issues));
                    } else if let Some(existing) =
                        library.rooms.iter().position(|r| r.id == room.id)
                    {
                        if semver(&room.version) > semver(&library.rooms[existing].version) {
                            library.rooms[existing] = room;
                        } else {
                            library.rejected.push((
                                at.clone(),
                                vec![LibraryIssue::new(
                                    at,
                                    format!(
                                        "{} does not replace a newer or equal version",
                                        room.id
                                    ),
                                )],
                            ));
                        }
                    } else {
                        library.rooms.push(room);
                    }
                }
                Err(issues) => library.rejected.push((at, issues)),
            }
        }
        library.roles.sort_by(|a, b| a.name.cmp(&b.name));
        library.rooms.sort_by(|a, b| a.name.cmp(&b.name));
        library
    }

    pub fn role(&self, id: &str) -> Option<&RoleCard> {
        self.roles.iter().find(|role| role.id == id)
    }

    pub fn room(&self, id: &str) -> Option<&RoomSpec> {
        self.rooms.iter().find(|room| room.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RESEARCH: &str = include_str!("../../../../docs/research/chat-rooms-and-roles.md");

    fn validator(file: &str) -> jsonschema::Validator {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../schemas")
            .join(file);
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        let schema: serde_json::Value = serde_json::from_str(&raw).expect("schema is JSON");
        jsonschema::validator_for(&schema).expect("schema compiles")
    }

    /// The whole built-in library loads: every file passes its JSON Schema
    /// AND the rules a schema cannot say, and nothing is rejected.
    #[test]
    fn every_builtin_passes_its_schema_and_its_rules() {
        let roles = validator("role.v1.schema.json");
        for (id, text) in BUILTIN_ROLES {
            let doc: serde_json::Value = serde_json::from_str(text).expect("role is JSON");
            let errors: Vec<String> = roles.iter_errors(&doc).map(|e| e.to_string()).collect();
            assert!(errors.is_empty(), "role {id}: {errors:?}");
        }
        let rooms = validator("room.v1.schema.json");
        for (id, text) in BUILTIN_ROOMS {
            let doc: serde_json::Value = serde_json::from_str(text).expect("room is JSON");
            let errors: Vec<String> = rooms.iter_errors(&doc).map(|e| e.to_string()).collect();
            assert!(errors.is_empty(), "room {id}: {errors:?}");
        }
        let library = Library::builtin();
        assert!(library.rejected.is_empty(), "{:?}", library.rejected);
        assert_eq!(library.roles.len(), BUILTIN_ROLES.len());
        assert_eq!(library.rooms.len(), BUILTIN_ROOMS.len());
        for id in ["dreamer", "realist", "skeptic", "selector"] {
            assert!(library.role(id).is_some(), "the operator's named role {id}");
        }
    }

    /// Every source a built-in cites is in the research page, so a role's
    /// evidence can always be followed.
    #[test]
    fn every_cited_source_is_in_the_research() {
        let library = Library::builtin();
        let provenances = library
            .roles
            .iter()
            .map(|role| (role.id.as_str(), &role.provenance))
            .chain(
                library
                    .rooms
                    .iter()
                    .map(|room| (room.id.as_str(), &room.provenance)),
            );
        for (id, provenance) in provenances {
            for source in &provenance.sources {
                let numbered = format!("\n{source}. ");
                let lettered = format!("\n- {source} ");
                assert!(
                    RESEARCH.contains(&numbered) || RESEARCH.contains(&lettered),
                    "{id} cites [{source}], which the research page does not list"
                );
            }
        }
    }

    fn selector_text() -> String {
        BUILTIN_ROLES
            .iter()
            .find(|(id, _)| *id == "selector")
            .map(|(_, text)| (*text).to_owned())
            .expect("selector")
    }

    /// Select, never blend: a selecting role that may merge is refused.
    #[test]
    fn the_plain_names_are_the_schema_names() {
        for stance in [
            Stance::Generate,
            Stance::Ground,
            Stance::Critique,
            Stance::Adversarial,
            Stance::Plan,
            Stance::Select,
        ] {
            assert_eq!(serde_json::to_value(stance).unwrap(), stance.as_str());
        }
        for kind in [
            RoomType::Council,
            RoomType::Debate,
            RoomType::AuthorCritic,
            RoomType::Router,
            RoomType::Brainstorm,
        ] {
            assert_eq!(serde_json::to_value(kind).unwrap(), kind.as_str());
        }
    }

    #[test]
    fn a_selector_that_may_merge_is_refused() {
        let mut role: RoleCard = serde_json::from_str(&selector_text()).expect("selector");
        role.must_not.retain(|rule| !rule.contains("merge"));
        let issues = validate_role(&role);
        assert!(
            issues
                .iter()
                .any(|issue| issue.problem.contains("select, never blend")),
            "{issues:?}"
        );
    }

    /// The operator's own files: a broken one is named and skipped; a higher
    /// version replaces a built-in; an equal version does not.
    #[test]
    fn user_files_are_validated_and_only_a_higher_version_replaces_a_builtin() {
        let mut newer: serde_json::Value =
            serde_json::from_str(&selector_text()).expect("selector");
        newer["version"] = "1.1.0".into();
        newer["summary"] = "The operator's own selector.".into();
        let user_roles = vec![
            ("broken.json".to_owned(), "{ not json".to_owned()),
            ("selector-newer.json".to_owned(), newer.to_string()),
            ("selector-same.json".to_owned(), selector_text()),
        ];
        let library = Library::with_user(&user_roles, &[]);
        assert_eq!(
            library.role("selector").map(|role| role.summary.as_str()),
            Some("The operator's own selector.")
        );
        let rejected: Vec<&str> = library.rejected.iter().map(|(at, _)| at.as_str()).collect();
        assert_eq!(rejected, vec!["broken.json", "selector-same.json"]);
    }

    /// A room is checked against the library it would seat.
    #[test]
    fn a_room_must_seat_real_roles_and_fill_their_parameters() {
        let library = Library::builtin();
        let mut room = library.room("council").expect("council").clone();
        room.seats[0].parameters.clear();
        room.seats.push(Seat {
            seat: "ghost".to_owned(),
            role: "oracle@1".to_owned(),
            count: 1,
            parameters: BTreeMap::new(),
        });
        let problems: Vec<String> = validate_room(&room, &library.roles)
            .into_iter()
            .map(|issue| issue.problem)
            .collect();
        assert!(
            problems
                .iter()
                .any(|p| p.contains("must fill domain-expert's domain")),
            "{problems:?}"
        );
        assert!(
            problems.iter().any(|p| p.contains("oracle@1")),
            "{problems:?}"
        );
    }

    #[test]
    fn a_rooms_selection_seat_must_be_a_selector() {
        let library = Library::builtin();
        let mut room = library.room("council").expect("council").clone();
        if let Some(selection) = room.selection.as_mut() {
            selection.seat = "answerer".to_owned();
        }
        assert!(validate_room(&room, &library.roles)
            .iter()
            .any(|issue| issue.problem.contains("must be a selecting role")));
    }
}
