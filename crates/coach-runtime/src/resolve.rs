//! Path + fingerprint helpers shared by unit tests and Linux resolve.

use crate::NodeRef;
use dpl_coach_fabric::Refused;

/// Parse fabric `path` (`0` or `0/1/2…`) into child indices after the root.
pub fn path_indices(path: &str) -> Result<Vec<u32>, Refused> {
    if path.is_empty() {
        return Err(Refused {
            reason: "empty fabric path".into(),
            tip: "Re-import the app to rebuild the signal map.".into(),
        });
    }
    let mut parts = Vec::new();
    for (i, seg) in path.split('/').enumerate() {
        let Ok(n) = seg.parse::<u32>() else {
            return Err(Refused {
                reason: format!("invalid fabric path segment {seg:?} in {path:?}"),
                tip: "Re-import the app — the signal map path is corrupt.".into(),
            });
        };
        if i == 0 {
            if n != 0 {
                return Err(Refused {
                    reason: format!("fabric path must start at root 0, got {path:?}"),
                    tip: "Re-import the app to rebuild the signal map.".into(),
                });
            }
            continue;
        }
        parts.push(n);
    }
    Ok(parts)
}

/// Name must match; role match is lenient (substring / normalized).
pub fn fingerprint_matches(expected: &NodeRef, name: &str, role: &str) -> bool {
    if name != expected.name {
        return false;
    }
    if expected.role.is_empty() || role.is_empty() {
        return true;
    }
    let er = normalize_role(&expected.role);
    let ar = normalize_role(role);
    er == ar || ar.contains(&er) || er.contains(&ar)
}

fn normalize_role(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}
