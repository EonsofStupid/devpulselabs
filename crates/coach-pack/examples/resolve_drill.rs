//! The S3a live drill — resolve THIS seat's real inventory against a
//! real registry on disk.
//!
//! ```text
//! cargo run -p dpl-coach-pack --example resolve_drill -- <registry-root> [seed-blender] [seed-calculator]
//! ```
//!
//! The seed args first write real packs through the crate's own write
//! path, then the scan resolves every inventory row: rows a pack claims
//! resolve to their minted `AppId` with the matched evidence shown;
//! strangers synthesize honest `x.unresolved:` profiles.

use dpl_coach_pack::{
    FingerprintBundle, Identity, Pack, PackRegistry, Resolution, CURRENT_SEAM, PACK_SCHEMA,
};
// Identity types live at their one home since S2 (no re-export shims).
use egui_pilot_ai_protocol::seam::{AppId, CellKey};

fn blender_pack() -> Pack {
    Pack {
        schema: PACK_SCHEMA,
        min_seam: CURRENT_SEAM,
        identity: Identity {
            app_id: AppId::parse("org.blender.Blender").expect("minted id"),
            display_name: "Blender".into(),
            desktop_ids: vec![
                "blender.desktop".into(),
                "org.blender.Blender.desktop".into(),
            ],
            wm_classes: vec!["Blender".into(), "blender".into()],
            atspi_names: vec!["Blender".into()],
            exe_basenames: vec!["blender".into()],
            aliases: vec![],
        },
        cell: CellKey::this_machine("4.2"),
        ladder: Vec::new(),
        verbs: Vec::new(),
        teaching_landmarks: Vec::new(),
        extras: toml::Table::new(),
    }
}

fn calculator_pack() -> Pack {
    Pack {
        schema: PACK_SCHEMA,
        min_seam: CURRENT_SEAM,
        identity: Identity {
            app_id: AppId::parse("org.gnome.Calculator").expect("minted id"),
            display_name: "Calculator".into(),
            desktop_ids: vec!["org.gnome.Calculator.desktop".into()],
            wm_classes: vec!["gnome-calculator".into()],
            atspi_names: vec!["gnome-calculator".into()],
            exe_basenames: vec!["gnome-calculator".into()],
            aliases: vec![],
        },
        cell: CellKey::this_machine("48"),
        ladder: Vec::new(),
        verbs: Vec::new(),
        teaching_landmarks: Vec::new(),
        extras: toml::Table::new(),
    }
}

fn describe(resolution: &Resolution) -> String {
    match resolution {
        Resolution::Resolved {
            app_id,
            score,
            matched,
        } => {
            let evidence: Vec<String> = matched
                .iter()
                .map(|m| format!("{}={} (+{})", m.signal, m.value, m.weight))
                .collect();
            format!(
                "RESOLVED {app_id}  score {score}  [{}]",
                evidence.join(", ")
            )
        }
        Resolution::Unresolved { synthesized } => {
            format!("unresolved → synthesized {}", synthesized.identity.app_id)
        }
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(root) = args.next() else {
        eprintln!("usage: resolve_drill <registry-root> [seed-blender]");
        std::process::exit(2);
    };
    let registry = PackRegistry::open(&root);

    for seed in args {
        let pack = match seed.as_str() {
            "seed-blender" => blender_pack(),
            "seed-calculator" => calculator_pack(),
            other => {
                eprintln!("unknown arg {other:?} (want seed-blender | seed-calculator)");
                std::process::exit(2);
            }
        };
        let path = registry.write(&pack).expect("write pack");
        println!("seeded {}", path.display());
    }

    let installed = registry.list();
    println!(
        "registry {root}: {} pack(s) installed: {}",
        installed.len(),
        installed
            .iter()
            .map(AppId::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    );

    let items = match dpl_coach_inventory::inventory(true) {
        Ok(items) => items,
        Err(e) => {
            println!("live census unavailable ({e}) — cold catalog only");
            dpl_coach_inventory::inventory(false).expect("cold inventory")
        }
    };
    println!("{} inventory rows on this seat\n", items.len());

    let mut resolved = 0usize;
    let mut synthesized = 0usize;
    for item in &items {
        let bundle = FingerprintBundle::from_inventory(item);
        let resolution = registry.resolve(&bundle);
        match resolution {
            Resolution::Resolved { .. } => resolved += 1,
            Resolution::Unresolved { .. } => synthesized += 1,
        }
        println!("{:<40} {}", item.display_name, describe(&resolution));
    }
    println!("\n{resolved} resolved · {synthesized} synthesized (honest strangers)");

    // Blender is not installed on every seat — prove the pack matches a
    // Blender-shaped bundle exactly as its .desktop would present it.
    let blender_bundle = FingerprintBundle {
        desktop_id: Some("blender.desktop".into()),
        startup_wm_class: Some("blender".into()),
        exe_basename: Some("blender".into()),
        ..Default::default()
    };
    println!(
        "\nblender-shaped bundle: {}",
        describe(&registry.resolve(&blender_bundle))
    );
}
