//! The headless index drill: `dpl-index-cli [--root <dir>] <reindex|list [kind]|recent [n]>`.
//!
//! Exercises the REAL engine against a real root — the engine-tier drill
//! the pilot law asks for (UI scenarios ride X2).

use std::path::PathBuf;

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (root, userdata, rest): (PathBuf, PathBuf, Vec<String>) =
        match args.first().map(String::as_str) {
            Some("--root") => {
                let Some(dir) = args.get(1) else {
                    eprintln!("--root needs a directory");
                    return 2;
                };
                let root = PathBuf::from(dir);
                let userdata = dpl_roots::ring_sibling(&root, ".userdata");
                (root, userdata, args[2..].to_vec())
            }
            _ => match dpl_roots::initialize() {
                Ok(layout) => (layout.content, layout.userdata, args),
                Err(error) => {
                    eprintln!("{error}");
                    return 1;
                }
            },
        };

    // The ring law (ADR-014): this root's index lives in its sibling
    // `.userdata/`, adopting a legacy in-root `artifacts.db` on first use.
    let db = userdata.join("index.db");
    let index = match dpl_index::Index::open_adopting(&db, &root.join("artifacts.db")) {
        Ok(ix) => ix,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };

    let result = match rest.first().map(String::as_str) {
        Some("reindex") => index.reindex(&root).map(|r| {
            println!("indexed {} · soft-cleared {}", r.indexed, r.cleared);
        }),
        Some("list") => index
            .list(rest.get(1).map(String::as_str), None)
            .map(print_rows),
        Some("recent") => {
            let n = rest.get(1).and_then(|s| s.parse().ok()).unwrap_or(20);
            index.recent(n, None).map(print_rows)
        }
        Some(op @ ("keep" | "associate")) => {
            let (Some(rel), Some(show), Some(ep)) = (rest.get(1), rest.get(2), rest.get(3)) else {
                eprintln!("usage: dpl-index-cli [--root <dir>] {op} <rel_path> <show> <episode>");
                return 2;
            };
            let run = if op == "keep" {
                index.keep(&root, rel, show, ep)
            } else {
                index.associate(&root, rel, show, ep)
            };
            run.map(|new_rel| println!("{op}: {rel} → {new_rel}"))
        }
        _ => {
            eprintln!(
                "usage: dpl-index-cli [--root <dir>] <reindex | list [kind] | recent [n] | \
                 keep <rel> <show> <ep> | associate <rel> <show> <ep>>"
            );
            return 2;
        }
    };
    match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

fn print_rows(rows: Vec<dpl_index::Record>) {
    for r in &rows {
        println!(
            "{:<7} {:<9} {:<28} {}",
            r.kind, r.location, r.name, r.rel_path
        );
    }
    println!("{} artifacts", rows.len());
}
