//! CLI proof for C2d — cold (+ optional live) inventory list.
//!
//! Plan: `docs/plans/coach-c2d-lightweight-inventory.md`

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let include_live = !args.iter().any(|a| a == "--no-live");
    let json = args.iter().any(|a| a == "--json");

    match dpl_coach_inventory::inventory(include_live) {
        Ok(items) => {
            if json {
                // Minimal hand-rolled JSON to avoid a serde dep on this crate.
                println!("[");
                for (i, item) in items.iter().enumerate() {
                    let live = item
                        .live
                        .as_ref()
                        .map(|l| {
                            format!(
                                r#"{{"atspi_name":{},"pid":{},"running":{}}}"#,
                                json_str(&l.atspi_name),
                                l.pid
                                    .map(|p| p.to_string())
                                    .unwrap_or_else(|| "null".into()),
                                l.running
                            )
                        })
                        .unwrap_or_else(|| "null".into());
                    println!(
                        r#"  {{"id":{},"display_name":{},"exec_hint":{},"cold_ok":{},"complexity":{},"running":{},"live":{},"seed_tip":{}}}{}"#,
                        json_str(item.id.as_str()),
                        json_str(&item.display_name),
                        json_str(&item.exec_hint),
                        item.cold_ok,
                        json_str(item.complexity.as_str()),
                        item.live.is_some(),
                        live,
                        item.seed_tip
                            .as_deref()
                            .map(json_str)
                            .unwrap_or_else(|| "null".into()),
                        if i + 1 == items.len() { "" } else { "," }
                    );
                }
                println!("]");
            } else {
                println!(
                    "coach-inventory-list: {} item(s) · live={}",
                    items.len(),
                    include_live
                );
                for item in &items {
                    let run = if item.live.is_some() { "RUNNING" } else { "cold" };
                    let tip = item.seed_tip.as_deref().unwrap_or("");
                    println!(
                        "{:40}  {:28}  {}  {}  {}",
                        item.id.as_str(),
                        item.display_name,
                        item.complexity.as_str(),
                        run,
                        tip
                    );
                }
            }
        }
        Err(e) => {
            eprintln!("coach-inventory-list: {e}");
            std::process::exit(2);
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
