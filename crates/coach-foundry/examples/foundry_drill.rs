//! The Foundry drill — the committed terminal proof for S3b (plan §7):
//! run the staged pipeline against a LIVE app, resume after a simulated
//! kill, purge, remake. Usage:
//!
//! ```text
//! cargo run -p dpl-coach-foundry --example foundry_drill -- \
//!     <selector> <registry-root> <analytics-root> [halt-after=<stage>] [purge] [remake]
//! ```
//!
//! Examples (calculator must be running):
//!
//! ```text
//! foundry_drill Calculator /tmp/s3b-reg /tmp/s3b-ana
//! foundry_drill Calculator /tmp/s3b-reg /tmp/s3b-ana halt-after=fabric
//! foundry_drill Calculator /tmp/s3b-reg /tmp/s3b-ana purge
//! foundry_drill Calculator /tmp/s3b-reg /tmp/s3b-ana remake
//! ```

use dpl_coach_foundry::{
    mint_app_id_from_evidence, purge, remake, run, FoundryOutcome, FoundryRequest, FoundryStage,
};
use dpl_coach_pack::{FingerprintBundle, PackRegistry, Resolution};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 3 {
        eprintln!("usage: foundry_drill <selector> <registry-root> <analytics-root> [halt-after=<stage>] [purge] [remake]");
        std::process::exit(2);
    }
    let selector = args[0].clone();
    let mut request = FoundryRequest::new(&selector, &args[1], &args[2]);
    let mut do_purge = false;
    let mut do_remake = false;
    for arg in &args[3..] {
        if let Some(slug) = arg.strip_prefix("halt-after=") {
            request.halt_after = FoundryStage::parse(slug);
            if request.halt_after.is_none() {
                eprintln!("unknown stage {slug:?}");
                std::process::exit(2);
            }
        } else if arg == "purge" {
            do_purge = true;
        } else if arg == "remake" {
            do_remake = true;
        } else {
            eprintln!("unknown arg {arg:?}");
            std::process::exit(2);
        }
    }

    let progress = |report: dpl_coach_foundry::StageReport| {
        println!(
            "[{:>7}] {:?} — {}",
            report.stage.slug(),
            report.status,
            report.detail
        );
    };

    if do_purge {
        // Purge needs the resolved id — resolve the selector's evidence.
        let registry = PackRegistry::open(&args[1]);
        let item = dpl_coach_fabric::find_inventory_item(&selector).unwrap_or_else(|r| {
            eprintln!("purge: {} — {}", r.reason, r.tip);
            std::process::exit(1);
        });
        let bundle = FingerprintBundle::from_inventory(&item);
        let id = match registry.resolve(&bundle) {
            Resolution::Resolved { app_id, .. } => app_id,
            Resolution::Unresolved { .. } => match mint_app_id_from_evidence(&bundle) {
                Some(id) => id,
                None => {
                    eprintln!("purge: {selector:?} resolves to nothing purgeable");
                    std::process::exit(1);
                }
            },
        };
        match purge(std::path::Path::new(&args[1]), &id) {
            Ok(trash) => println!("purged {} → {}", id, trash.display()),
            Err(e) => {
                eprintln!("purge: {e}");
                std::process::exit(1);
            }
        }
        return;
    }

    let outcome = if do_remake {
        let registry = PackRegistry::open(&args[1]);
        let item = dpl_coach_fabric::find_inventory_item(&selector).unwrap_or_else(|r| {
            eprintln!("remake: {} — {}", r.reason, r.tip);
            std::process::exit(1);
        });
        let bundle = FingerprintBundle::from_inventory(&item);
        let id = match registry.resolve(&bundle) {
            Resolution::Resolved { app_id, .. } => app_id,
            Resolution::Unresolved { .. } => match mint_app_id_from_evidence(&bundle) {
                Some(id) => id,
                None => {
                    eprintln!("remake: {selector:?} resolves to no identity");
                    std::process::exit(1);
                }
            },
        };
        remake(&request, &id, progress)
    } else {
        run(&request, progress)
    };

    match outcome {
        FoundryOutcome::Published {
            app_id,
            pack_path,
            warnings,
        } => {
            println!("PUBLISHED {app_id} → {}", pack_path.display());
            for w in warnings {
                println!("  warn: {w}");
            }
        }
        FoundryOutcome::Halted { after } => {
            println!(
                "HALTED after {} (test hook) — rerun to resume",
                after.slug()
            );
        }
        FoundryOutcome::Unresolved { display_name, tip } => {
            println!("UNRESOLVED {display_name} — {tip}");
            std::process::exit(1);
        }
        FoundryOutcome::Refused { stage, reason } => {
            println!("REFUSED at {}: {reason}", stage.slug());
            std::process::exit(1);
        }
    }
}
