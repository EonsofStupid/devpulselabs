//! The evidence resolver — scores a scan's fingerprint bundle against
//! every installed pack's `[identity]`, aliasing whatever was actually
//! observed onto the minted source-of-truth `AppId`. No match is an
//! honest miss: a generic profile is synthesized, never a guess.

use crate::bundle::FingerprintBundle;
use crate::id::{AppId, CellKey};
use crate::pack::{Identity, Pack, CURRENT_SEAM, PACK_SCHEMA};

/// Signal weights v1 (plan §4) — the ONE list; `score_bundle` consults
/// nothing else and the census test holds this to the bundle's shape.
/// The name is one weighted signal, never the anchor (locale law).
pub const SIGNAL_WEIGHTS: &[(&str, f32)] = &[
    ("desktop_id", 4.0),
    ("startup_wm_class", 3.0),
    ("wm_class", 3.0),
    ("atspi_name", 2.0),
    ("exe_basename", 2.0),
    ("alias", 1.0),
];

/// Accept at one strong signal or two weak corroborating ones.
pub const ACCEPT_SCORE: f32 = 3.0;

fn weight_of(signal: &str) -> f32 {
    SIGNAL_WEIGHTS
        .iter()
        .find(|(s, _)| *s == signal)
        .map(|(_, w)| *w)
        .unwrap_or(0.0)
}

/// One piece of matched evidence — kept so the UI and the ledger can
/// show WHY an app resolved, not just that it did.
#[derive(Debug, Clone, PartialEq)]
pub struct SignalMatch {
    pub signal: &'static str,
    pub value: String,
    pub weight: f32,
}

/// The resolver's verdict.
#[derive(Debug, Clone, PartialEq)]
pub enum Resolution {
    /// Evidence reached [`ACCEPT_SCORE`] against exactly one best pack.
    Resolved {
        app_id: AppId,
        score: f32,
        matched: Vec<SignalMatch>,
    },
    /// No pack claims this bundle (or the evidence tied). The seam never
    /// refuses to look: a promotable generic profile rides along.
    Unresolved { synthesized: Box<Pack> },
}

/// Score one bundle against one pack identity.
pub fn score_bundle(bundle: &FingerprintBundle, identity: &Identity) -> (f32, Vec<SignalMatch>) {
    let mut matched: Vec<SignalMatch> = Vec::new();
    let mut hit = |signal: &'static str, value: &str| {
        matched.push(SignalMatch {
            signal,
            value: value.to_string(),
            weight: weight_of(signal),
        });
    };
    if let Some(v) = &bundle.desktop_id {
        if contains_ci(&identity.desktop_ids, v) {
            hit("desktop_id", v);
        }
    }
    if let Some(v) = &bundle.startup_wm_class {
        if contains_ci(&identity.wm_classes, v) {
            hit("startup_wm_class", v);
        }
    }
    if let Some(v) = &bundle.wm_class {
        if contains_ci(&identity.wm_classes, v) {
            hit("wm_class", v);
        }
    }
    if let Some(v) = &bundle.atspi_name {
        if contains_ci(&identity.atspi_names, v) {
            hit("atspi_name", v);
        }
    }
    if let Some(v) = &bundle.exe_basename {
        if contains_ci(&identity.exe_basenames, v) {
            hit("exe_basename", v);
        }
    }
    // Aliases catch renames/wrappers: any observed string may match one.
    for v in [
        &bundle.desktop_id,
        &bundle.startup_wm_class,
        &bundle.wm_class,
        &bundle.atspi_name,
        &bundle.exe_basename,
    ]
    .into_iter()
    .flatten()
    {
        if contains_ci(&identity.aliases, v) {
            hit("alias", v);
            break;
        }
    }
    let score = matched.iter().map(|m| m.weight).sum();
    (score, matched)
}

/// Resolve a bundle against installed packs. Ties (same score AND same
/// evidence count) are ambiguity, and ambiguity is honest: `Unresolved`,
/// never a coin flip.
pub fn resolve_against<'a>(
    bundle: &FingerprintBundle,
    packs: impl IntoIterator<Item = &'a Pack>,
) -> Resolution {
    let mut best: Option<(f32, Vec<SignalMatch>, AppId)> = None;
    let mut tied = false;
    for pack in packs {
        let (score, matched) = score_bundle(bundle, &pack.identity);
        if score < ACCEPT_SCORE {
            continue;
        }
        match &best {
            None => best = Some((score, matched, pack.identity.app_id.clone())),
            Some((best_score, best_matched, _)) => {
                let better = score > *best_score
                    || (score == *best_score && matched.len() > best_matched.len());
                let equal = score == *best_score && matched.len() == best_matched.len();
                if better {
                    best = Some((score, matched, pack.identity.app_id.clone()));
                    tied = false;
                } else if equal {
                    tied = true;
                }
            }
        }
    }
    match best {
        Some((score, matched, app_id)) if !tied => Resolution::Resolved {
            app_id,
            score,
            matched,
        },
        _ => Resolution::Unresolved {
            synthesized: Box::new(synthesize_generic(bundle)),
        },
    }
}

/// A generic profile for an unclaimed app: default ladder only (a11y +
/// injection — nothing to promote), the observed evidence carried into
/// `[identity]` so a later Foundry promotion starts from facts.
pub fn synthesize_generic(bundle: &FingerprintBundle) -> Pack {
    let name = bundle.best_name().unwrap_or("unknown-app").to_string();
    let mut wm_classes: Vec<String> = Vec::new();
    for wm in [&bundle.startup_wm_class, &bundle.wm_class]
        .into_iter()
        .flatten()
    {
        if !wm_classes.iter().any(|w| w.eq_ignore_ascii_case(wm)) {
            wm_classes.push(wm.clone());
        }
    }
    Pack {
        schema: PACK_SCHEMA,
        min_seam: CURRENT_SEAM,
        identity: Identity {
            app_id: AppId::unresolved(&name),
            display_name: name,
            desktop_ids: bundle.desktop_id.iter().cloned().collect(),
            wm_classes,
            atspi_names: bundle.atspi_name.iter().cloned().collect(),
            exe_basenames: bundle.exe_basename.iter().cloned().collect(),
            aliases: Vec::new(),
        },
        cell: CellKey::this_machine(
            bundle
                .version
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
        ),
        ladder: Vec::new(),
        verbs: Vec::new(),
        teaching_landmarks: Vec::new(),
        extras: toml::Table::new(),
    }
}

fn contains_ci(haystack: &[String], needle: &str) -> bool {
    haystack.iter().any(|h| h.eq_ignore_ascii_case(needle))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pack::{PromotedRung, VerbGrant};

    fn pack(app_id: &str, identity: Identity) -> Pack {
        let mut identity = identity;
        identity.app_id = AppId::parse(app_id).unwrap();
        Pack {
            schema: PACK_SCHEMA,
            min_seam: CURRENT_SEAM,
            identity,
            cell: CellKey::new("1", "linux", "en"),
            ladder: Vec::new(),
            verbs: Vec::new(),
            teaching_landmarks: Vec::new(),
            extras: toml::Table::new(),
        }
    }

    fn blank_identity() -> Identity {
        Identity {
            app_id: AppId::parse("org.x.Placeholder").unwrap(),
            display_name: String::new(),
            desktop_ids: Vec::new(),
            wm_classes: Vec::new(),
            atspi_names: Vec::new(),
            exe_basenames: Vec::new(),
            aliases: Vec::new(),
        }
    }

    fn blender_packs() -> Vec<Pack> {
        vec![
            pack(
                "org.blender.Blender",
                Identity {
                    display_name: "Blender".into(),
                    desktop_ids: vec![
                        "blender.desktop".into(),
                        "org.blender.Blender.desktop".into(),
                    ],
                    wm_classes: vec!["Blender".into()],
                    atspi_names: vec!["Blender".into()],
                    exe_basenames: vec!["blender".into()],
                    ..blank_identity()
                },
            ),
            pack(
                "org.gnome.Calculator",
                Identity {
                    display_name: "Calculator".into(),
                    desktop_ids: vec!["org.gnome.Calculator.desktop".into()],
                    wm_classes: vec!["gnome-calculator".into()],
                    atspi_names: vec!["gnome-calculator".into()],
                    exe_basenames: vec!["gnome-calculator".into()],
                    ..blank_identity()
                },
            ),
        ]
    }

    #[test]
    fn weights_census_covers_every_evidence_signal_exactly() {
        // The exact v1 signal list — a new bundle field or weight edit
        // must amend this test consciously (census law).
        let signals: Vec<&str> = SIGNAL_WEIGHTS.iter().map(|(s, _)| *s).collect();
        assert_eq!(
            signals,
            vec![
                "desktop_id",
                "startup_wm_class",
                "wm_class",
                "atspi_name",
                "exe_basename",
                "alias"
            ]
        );
        // exe_path and version are deliberately NOT match signals in v1.
        assert!(SIGNAL_WEIGHTS.iter().all(|(_, w)| *w > 0.0));
    }

    #[test]
    fn blender_shaped_bundle_resolves_to_blender() {
        let bundle = FingerprintBundle {
            desktop_id: Some("blender.desktop".into()),
            startup_wm_class: Some("blender".into()), // case differs from pack
            exe_basename: Some("blender".into()),
            ..Default::default()
        };
        match resolve_against(&bundle, &blender_packs()) {
            Resolution::Resolved {
                app_id,
                score,
                matched,
            } => {
                assert_eq!(app_id.as_str(), "org.blender.Blender");
                assert_eq!(score, 4.0 + 3.0 + 2.0);
                assert_eq!(matched.len(), 3);
            }
            other => panic!("expected resolved, got {other:?}"),
        }
    }

    #[test]
    fn one_weak_signal_is_not_enough() {
        // Only an exe basename (2.0) — below ACCEPT_SCORE.
        let bundle = FingerprintBundle {
            exe_basename: Some("blender".into()),
            ..Default::default()
        };
        assert!(matches!(
            resolve_against(&bundle, &blender_packs()),
            Resolution::Unresolved { .. }
        ));
    }

    #[test]
    fn stranger_bundle_synthesizes_a_promotable_generic_profile() {
        let bundle = FingerprintBundle {
            atspi_name: Some("Mystery App".into()),
            exe_basename: Some("mystery".into()),
            ..Default::default()
        };
        let Resolution::Unresolved { synthesized } = resolve_against(&bundle, &blender_packs())
        else {
            panic!("expected unresolved");
        };
        assert_eq!(
            synthesized.identity.app_id.as_str(),
            "x.unresolved:mystery-app"
        );
        assert!(synthesized.identity.app_id.is_unresolved());
        assert_eq!(synthesized.identity.display_name, "Mystery App");
        assert_eq!(synthesized.identity.atspi_names, vec!["Mystery App"]);
        assert_eq!(synthesized.identity.exe_basenames, vec!["mystery"]);
        // Default ladder only — nothing granted, nothing to bypass.
        assert!(synthesized.ladder.is_empty());
        assert!(synthesized.verbs.is_empty());
        // The synthesized profile is itself a valid, serializable pack.
        assert!(synthesized.to_signed_toml().is_ok());
    }

    #[test]
    fn ambiguous_tie_is_honest_unresolved() {
        let twin = |id: &str| {
            pack(
                id,
                Identity {
                    desktop_ids: vec!["twin.desktop".into()],
                    ..blank_identity()
                },
            )
        };
        let packs = vec![twin("org.a.Twin"), twin("org.b.Twin")];
        let bundle = FingerprintBundle {
            desktop_id: Some("twin.desktop".into()),
            ..Default::default()
        };
        assert!(matches!(
            resolve_against(&bundle, &packs),
            Resolution::Unresolved { .. }
        ));
    }

    #[test]
    fn higher_evidence_wins_a_same_score_race() {
        // Same score, more matched signals → the richer pack wins.
        let a = pack(
            "org.a.App",
            Identity {
                desktop_ids: vec!["app.desktop".into()], // 4.0, one signal
                ..blank_identity()
            },
        );
        let b = pack(
            "org.b.App",
            Identity {
                exe_basenames: vec!["app".into()], // 2.0
                atspi_names: vec!["app".into()],   // 2.0 → 4.0, two signals
                ..blank_identity()
            },
        );
        let bundle = FingerprintBundle {
            desktop_id: Some("app.desktop".into()),
            exe_basename: Some("app".into()),
            atspi_name: Some("app".into()),
            ..Default::default()
        };
        // a scores 4.0/1 match, b scores 4.0/2 matches.
        match resolve_against(&bundle, [&a, &b]) {
            Resolution::Resolved { app_id, .. } => assert_eq!(app_id.as_str(), "org.b.App"),
            other => panic!("expected resolved, got {other:?}"),
        }
    }

    #[test]
    fn packs_with_grants_still_resolve_like_any_other() {
        let mut packs = blender_packs();
        packs[0]
            .ladder
            .push(("open".into(), PromotedRung::R0Native));
        packs[0].verbs.push(VerbGrant {
            verb: "open".into(),
            rung: PromotedRung::R0Native,
            reliability: Some(1.0),
            oracle: None,
            extras: toml::Table::new(),
        });
        let bundle = FingerprintBundle {
            desktop_id: Some("org.blender.Blender.desktop".into()),
            ..Default::default()
        };
        assert!(matches!(
            resolve_against(&bundle, &packs),
            Resolution::Resolved { .. }
        ));
    }
}
