//! The seven Foundry stages (seam ADR §3.5) and their checkpoint law.
//!
//! Census and Resolve are LIVE TRUTH — they re-run on every pass, because
//! presence and identity evidence can change between runs. Fabric, Probe,
//! Induct and Battery are EXPENSIVE — each leaves an artifact beside the
//! pack, and a resumed job skips any stage whose artifact survives.
//! Publish is idempotent (the registry write steps the old pack aside).

/// One stage of the Foundry pipeline, in execution order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoundryStage {
    /// Observe the live app: inventory row, fingerprint bundle, presence.
    Census,
    /// Score the bundle against installed packs; mint identity if the
    /// evidence itself names one (reverse-DNS desktop id).
    Resolve,
    /// Mint the anchor fabric (AT-SPI walk) and elect the seed control.
    Fabric,
    /// Execute the safe verb battery K times each — reliability is
    /// measured on the seam, every run an S1 span.
    Probe,
    /// Fold census + anchors + probe stats into a pack draft.
    Induct,
    /// Re-verify every granted verb once against the draft.
    Battery,
    /// Sign and write the pack cell into the registry.
    Publish,
}

impl FoundryStage {
    pub const ALL: [Self; 7] = [
        Self::Census,
        Self::Resolve,
        Self::Fabric,
        Self::Probe,
        Self::Induct,
        Self::Battery,
        Self::Publish,
    ];

    pub const fn slug(self) -> &'static str {
        match self {
            Self::Census => "census",
            Self::Resolve => "resolve",
            Self::Fabric => "fabric",
            Self::Probe => "probe",
            Self::Induct => "induct",
            Self::Battery => "battery",
            Self::Publish => "publish",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.slug() == raw)
    }

    /// The artifact whose presence marks this stage checkpointed —
    /// `None` for the live-truth and idempotent stages, which always run.
    pub const fn checkpoint_artifact(self) -> Option<&'static str> {
        match self {
            Self::Census | Self::Resolve | Self::Publish => None,
            Self::Fabric => Some("anchors.toml"),
            Self::Probe => Some("probes.toml"),
            Self::Induct => Some("pack.draft.toml"),
            Self::Battery => Some("battery.toml"),
        }
    }

    /// Layman copy for the hub's console rows.
    pub const fn human(self) -> &'static str {
        match self {
            Self::Census => "Observing the app",
            Self::Resolve => "Matching its identity",
            Self::Fabric => "Mapping the controls",
            Self::Probe => "Testing what we can drive",
            Self::Induct => "Writing the profile",
            Self::Battery => "Double-checking every verb",
            Self::Publish => "Publishing to the catalog",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_round_trip_in_order() {
        for stage in FoundryStage::ALL {
            assert_eq!(FoundryStage::parse(stage.slug()), Some(stage));
        }
        assert_eq!(FoundryStage::parse("smelt"), None);
    }

    #[test]
    fn checkpoint_law_expensive_stages_only() {
        let checkpointed: Vec<_> = FoundryStage::ALL
            .into_iter()
            .filter(|s| s.checkpoint_artifact().is_some())
            .collect();
        assert_eq!(
            checkpointed,
            vec![
                FoundryStage::Fabric,
                FoundryStage::Probe,
                FoundryStage::Induct,
                FoundryStage::Battery,
            ]
        );
    }
}
