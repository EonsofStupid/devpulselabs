//! Identity newtypes — the minted-once `AppId`, the first-class cell
//! key, and the derived `PackId`. No bare strings for identity.

use std::fmt;

/// The `x.` extension namespace for scan-observed apps no pack claims
/// yet (seam ADR §7.2).
pub const UNRESOLVED_PREFIX: &str = "x.unresolved:";

/// Stable application identity — minted once per pack, never derived at
/// runtime. Exactly two forms parse; everything else is refused:
///
/// - resolved: reverse-DNS with ≥ 2 segments (`org.blender.Blender`);
///   the `x.` first segment is reserved for extensions
/// - extension: `x.unresolved:<slug>` with a `[a-z0-9-]` slug
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AppId(String);

/// Why an id string was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdError {
    pub raw: String,
    pub reason: &'static str,
}

impl fmt::Display for IdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "app id {:?} refused: {}", self.raw, self.reason)
    }
}

impl std::error::Error for IdError {}

impl AppId {
    pub fn parse(raw: &str) -> Result<Self, IdError> {
        if let Some(slug) = raw.strip_prefix(UNRESOLVED_PREFIX) {
            if !slug.is_empty()
                && slug
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            {
                return Ok(Self(raw.to_string()));
            }
            return Err(IdError {
                raw: raw.to_string(),
                reason: "unresolved slug must be non-empty [a-z0-9-]",
            });
        }
        if raw.starts_with("x.") {
            return Err(IdError {
                raw: raw.to_string(),
                reason: "the x. namespace is reserved for extensions (x.unresolved:<slug>)",
            });
        }
        let segments: Vec<&str> = raw.split('.').collect();
        if segments.len() < 2 {
            return Err(IdError {
                raw: raw.to_string(),
                reason: "a resolved app id needs at least two dot segments",
            });
        }
        for segment in &segments {
            if segment.is_empty()
                || !segment
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-')
            {
                return Err(IdError {
                    raw: raw.to_string(),
                    reason: "segments must be non-empty [A-Za-z0-9-]",
                });
            }
        }
        Ok(Self(raw.to_string()))
    }

    /// Sanitize a scan-observed name into the extension namespace — the
    /// honest identity for an app no installed pack claims.
    pub fn unresolved(observed: &str) -> Self {
        Self(format!("{UNRESOLVED_PREFIX}{}", slugify(observed)))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_unresolved(&self) -> bool {
        self.0.starts_with(UNRESOLVED_PREFIX)
    }
}

impl fmt::Display for AppId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Lowercase, `[a-z0-9-]`, dashes collapsed and trimmed.
pub(crate) fn slugify(raw: &str) -> String {
    let mut out = String::new();
    let mut pending_dash = false;
    for c in raw.chars() {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            if pending_dash && !out.is_empty() {
                out.push('-');
            }
            pending_dash = false;
            out.push(c);
        } else {
            pending_dash = true;
        }
    }
    if out.is_empty() {
        "unknown-app".to_string()
    } else {
        out
    }
}

/// The pack cell key — app version × OS × locale, first-class from day
/// one (seam ADR §7.4) so multi-cell is never a retrofit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellKey {
    pub app_version: String,
    pub os: String,
    pub locale: String,
}

impl CellKey {
    pub fn new(
        app_version: impl Into<String>,
        os: impl Into<String>,
        locale: impl Into<String>,
    ) -> Self {
        Self {
            app_version: app_version.into(),
            os: os.into(),
            locale: locale.into(),
        }
    }

    /// The cell this process runs in: compile-target OS + env locale.
    pub fn this_machine(app_version: impl Into<String>) -> Self {
        Self::new(app_version, std::env::consts::OS, current_locale())
    }

    pub fn label(&self) -> String {
        format!("{}/{}/{}", self.app_version, self.os, self.locale)
    }
}

/// `LC_ALL` → `LC_MESSAGES` → `LANG`, language part only (`en_US.UTF-8`
/// → `en`). Absent locale is reported honestly, never guessed.
fn current_locale() -> String {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .find_map(|k| std::env::var(k).ok().filter(|v| !v.is_empty()))
        .map(|v| {
            v.split(['_', '.', '@'])
                .next()
                .unwrap_or("unknown")
                .to_lowercase()
        })
        .unwrap_or_else(|| "unknown".to_string())
}

/// Pack identity: the app id plus its cell — `<app-id>@<version>/<os>/<locale>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackId(String);

impl PackId {
    pub fn new(app: &AppId, cell: &CellKey) -> Self {
        Self(format!("{}@{}", app.as_str(), cell.label()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PackId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_id_parse_law_table() {
        // Accepted.
        assert!(AppId::parse("org.blender.Blender").is_ok());
        assert!(AppId::parse("org.gnome.Calculator").is_ok());
        assert!(AppId::parse("com.example.app-2").is_ok());
        assert!(AppId::parse("x.unresolved:mystery-app").is_ok());
        // Refused.
        assert!(AppId::parse("blender").is_err()); // one segment
        assert!(AppId::parse("").is_err());
        assert!(AppId::parse("org..blender").is_err()); // empty segment
        assert!(AppId::parse("org.blender/Blender").is_err()); // charset
        assert!(AppId::parse("x.blender.set-mode").is_err()); // reserved x.
        assert!(AppId::parse("x.unresolved:").is_err()); // empty slug
        assert!(AppId::parse("x.unresolved:Has Space").is_err()); // slug charset
    }

    #[test]
    fn unresolved_mints_a_valid_slugged_id() {
        let id = AppId::unresolved("Mystery App 3.1!");
        assert_eq!(id.as_str(), "x.unresolved:mystery-app-3-1");
        assert!(id.is_unresolved());
        // Whatever unresolved() mints must itself pass parse.
        assert_eq!(AppId::parse(id.as_str()).unwrap(), id);
        assert_eq!(
            AppId::unresolved("!!!").as_str(),
            "x.unresolved:unknown-app"
        );
    }

    #[test]
    fn resolved_ids_are_not_unresolved() {
        assert!(!AppId::parse("org.blender.Blender").unwrap().is_unresolved());
    }

    #[test]
    fn pack_id_carries_the_cell() {
        let app = AppId::parse("org.blender.Blender").unwrap();
        let cell = CellKey::new("4.2", "linux", "en");
        assert_eq!(
            PackId::new(&app, &cell).as_str(),
            "org.blender.Blender@4.2/linux/en"
        );
    }
}
