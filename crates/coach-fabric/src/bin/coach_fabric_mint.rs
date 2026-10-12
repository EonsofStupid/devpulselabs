//! CLI proof for C2e — budgeted fabric mint for one selected target.
//!
//! Plan: `docs-history-2026-10-12:docs/plans/coach-c2e-fabric-mint.md`
//!
//! ```text
//! coach-fabric-mint --name Calculator
//! coach-fabric-mint --id org.gnome.Calculator.desktop --json
//! coach-fabric-mint --name Calculator --budget-nodes 50
//! ```

use dpl_coach_fabric::{mint, request_from_inventory_name, MintBudget, MintOutcome, MintPhase};
use std::io::Write;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let json = args.iter().any(|a| a == "--json");
    let quiet = args.iter().any(|a| a == "--quiet");

    let target = args
        .windows(2)
        .find(|w| w[0] == "--name" || w[0] == "--id")
        .map(|w| w[1].clone())
        .unwrap_or_else(|| "Calculator".into());

    let budget_nodes = args
        .windows(2)
        .find(|w| w[0] == "--budget-nodes")
        .and_then(|w| w[1].parse::<u32>().ok());

    let mut request = match request_from_inventory_name(&target) {
        Ok(r) => r,
        Err(r) => {
            if json {
                println!(
                    r#"{{"outcome":"refused","reason":{},"tip":{}}}"#,
                    json_str(&r.reason),
                    json_str(&r.tip)
                );
            } else {
                eprintln!("coach-fabric-mint: refused — {}", r.reason);
                eprintln!("  tip: {}", r.tip);
            }
            std::process::exit(2);
        }
    };

    if let Some(n) = budget_nodes {
        let mut b = MintBudget::for_complexity(request.complexity);
        b.max_nodes = n;
        request.budget = Some(b);
    }

    if !json && !quiet {
        println!(
            "coach-fabric-mint: target={} atspi={} complexity={} seed={:?}",
            request.target_id.as_str(),
            request.atspi_name,
            request.complexity.as_str(),
            request.seed_name
        );
        println!("  {}", dpl_coach_fabric::eta_hint(request.complexity));
    }

    let outcome = mint(&request, None, |p| {
        if json || quiet {
            return;
        }
        if matches!(p.phase, MintPhase::Walking) && p.visited > 0 && !p.visited.is_multiple_of(25) {
            return;
        }
        let _ = writeln!(
            std::io::stderr(),
            "  [{:>8}] visited={}/{} depth={} — {}",
            p.phase.as_str(),
            p.visited,
            p.budget,
            p.depth,
            p.eta_hint
        );
    });

    match outcome {
        MintOutcome::Ready(f) => {
            if json {
                println!(
                    r#"{{"outcome":"ready","target_id":{},"atspi_name":{},"nodes":{},"seed_path":{},"minted_ms":{},"has_seed_7":{}}}"#,
                    json_str(f.target_id.as_str()),
                    json_str(&f.atspi_name),
                    f.nodes.len(),
                    f.seed_path
                        .as_deref()
                        .map(json_str)
                        .unwrap_or_else(|| "null".into()),
                    f.minted_ms,
                    f.nodes.iter().any(|n| n.name == "7")
                );
            } else {
                println!(
                    "READY · {} nodes · {} ms · seed_path={:?}",
                    f.nodes.len(),
                    f.minted_ms,
                    f.seed_path
                );
                let sample: Vec<_> = f
                    .nodes
                    .iter()
                    .filter(|n| !n.name.is_empty())
                    .take(12)
                    .collect();
                for n in sample {
                    println!("  {:20}  {:16}  {}", n.path, n.role, n.name);
                }
            }
        }
        MintOutcome::Refused(r) => {
            if json {
                println!(
                    r#"{{"outcome":"refused","reason":{},"tip":{}}}"#,
                    json_str(&r.reason),
                    json_str(&r.tip)
                );
            } else {
                println!("REFUSED — {}", r.reason);
                println!("  tip: {}", r.tip);
            }
            std::process::exit(2);
        }
        MintOutcome::Cancelled => {
            if json {
                println!(r#"{{"outcome":"cancelled"}}"#);
            } else {
                println!("CANCELLED");
            }
            std::process::exit(3);
        }
    }
}

fn json_str(s: &str) -> String {
    let mut out = String::from("\"");
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
