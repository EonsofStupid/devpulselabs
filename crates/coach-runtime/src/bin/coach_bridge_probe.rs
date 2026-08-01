//! CX-bridge-01 probe — mint → resolve → invoke (+ optional extents).
//!
//! ```text
//! coach-bridge-probe --name Calculator
//! coach-bridge-probe --name cursor --seed "Search Ctrl+K"
//! coach-bridge-probe --name cursor --agency teacher --invoke click
//! ```

#![forbid(unsafe_code)]

use dpl_coach_fabric::{mint, request_from_inventory_name, MintOutcome};
use dpl_coach_runtime::{extents, invoke, mount_spec, resolve, AgencyMode, FabricSession};

fn main() {
    let code = match run(std::env::args().skip(1).collect()) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("coach-bridge-probe: {e}");
            1
        }
    };
    std::process::exit(code);
}

fn run(args: Vec<String>) -> Result<(), String> {
    let mut name = String::new();
    let mut seed_override: Option<String> = None;
    let mut agency = AgencyMode::Coach;
    let mut do_invoke = true;
    let mut no_seed = false;
    let mut action = "click".to_string();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--name" => {
                i += 1;
                name = args.get(i).cloned().ok_or("--name needs a value")?;
            }
            "--seed" => {
                i += 1;
                seed_override = Some(args.get(i).cloned().ok_or("--seed needs a value")?);
            }
            "--agency" => {
                i += 1;
                let v = args.get(i).map(String::as_str).unwrap_or("");
                agency = match v {
                    "coach" => AgencyMode::Coach,
                    "teacher" => AgencyMode::Teacher,
                    other => return Err(format!("unknown agency {other:?}")),
                };
            }
            "--action" => {
                i += 1;
                action = args.get(i).cloned().ok_or("--action needs a value")?;
            }
            "--no-invoke" => do_invoke = false,
            "--no-seed" => no_seed = true,
            "--help" | "-h" => {
                println!(
                    "usage: coach-bridge-probe --name <app> [--seed <name>] [--no-seed] [--agency coach|teacher] [--action click] [--no-invoke]"
                );
                return Ok(());
            }
            other => return Err(format!("unknown arg {other}")),
        }
        i += 1;
    }
    if name.is_empty() {
        return Err("--name required (Calculator or cursor)".into());
    }

    let mut request = request_from_inventory_name(&name)
        .map_err(|r| format!("REFUSED mint-request — {} · tip: {}", r.reason, r.tip))?;
    if no_seed {
        request.seed_name = None;
    } else if let Some(seed) = seed_override {
        request.seed_name = Some(seed);
    } else if request.seed_name.is_none() {
        let blob = format!("{} {}", request.target_id.as_str(), request.atspi_name).to_lowercase();
        if blob.contains("cursor") {
            request.seed_name = Some("Search Ctrl+K".into());
        }
    }

    println!(
        "mint: target={} atspi={} seed={:?} agency={}",
        request.target_id.as_str(),
        request.atspi_name,
        request.seed_name,
        agency.as_str()
    );

    let outcome = mint(&request, None, |p| {
        if p.visited == 0 || p.visited.is_multiple_of(50) || p.phase.as_str() == "done" {
            eprintln!(
                "  [{}] {}/{} {}",
                p.phase.as_str(),
                p.visited,
                p.budget,
                p.eta_hint
            );
        }
    });

    let fabric = match outcome {
        MintOutcome::Ready(f) => f,
        MintOutcome::Refused(r) => {
            return Err(format!("REFUSED mint — {} · tip: {}", r.reason, r.tip));
        }
        MintOutcome::Cancelled => return Err("mint cancelled".into()),
    };

    println!(
        "READY · {} nodes · {} ms · seed_path={:?}",
        fabric.nodes.len(),
        fabric.minted_ms,
        fabric.seed_path
    );
    if let Some(path) = &fabric.seed_path {
        if let Some(n) = fabric.nodes.iter().find(|n| n.path == *path) {
            println!(
                "seed_node: path={} role={} name={:?} actions={:?}",
                n.path, n.role, n.name, n.actions
            );
        }
    } else {
        // Diagnostic: named interactables (no seed gate).
        let interesting: Vec<_> = fabric
            .nodes
            .iter()
            .filter(|n| {
                !n.name.is_empty()
                    && (n.role.contains("button")
                        || n.role.contains("menu")
                        || n.name.contains("Search")
                        || n.name == "File")
            })
            .take(24)
            .collect();
        println!("named_interactables (sample {}):", interesting.len());
        for n in interesting {
            println!(
                "  path={} role={} name={:?} actions={:?}",
                n.path, n.role, n.name, n.actions
            );
        }
        println!("CX-bridge-01 probe: READY without seed (resolve skipped)");
        return Ok(());
    }

    let session = FabricSession::new(fabric, agency);
    let handle = resolve(
        &session,
        &session
            .seed_node()
            .ok_or_else(|| "no seed node after Ready".to_string())?,
    )
    .map_err(|r| format!("REFUSED resolve — {} · tip: {}", r.reason, r.tip))?;
    println!(
        "RESOLVED · path={} name={:?} role={}",
        handle.node.path, handle.node.name, handle.node.role
    );

    match extents(&handle) {
        Ok(r) => {
            let spec = mount_spec(&session, &handle)
                .map_err(|e| format!("mount_spec: {} · tip: {}", e.reason, e.tip))?;
            println!(
                "EXTENTS · window={} · mount_app={} expected={:?}",
                r.as_csv(),
                spec.atspi_name,
                spec.expected_name
            );
        }
        Err(r) => println!("EXTENTS refused — {} · tip: {}", r.reason, r.tip),
    }

    if do_invoke {
        let effect = invoke(&handle, &action, agency)
            .map_err(|r| format!("REFUSED invoke — {} · tip: {}", r.reason, r.tip))?;
        println!(
            "INVOKED · action={} agency={}",
            effect.action,
            effect.agency.as_str()
        );
    } else {
        println!("invoke skipped (--no-invoke)");
    }

    println!("CX-bridge-01 probe: PASS");
    Ok(())
}
