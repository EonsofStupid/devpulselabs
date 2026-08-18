//! The S1 lens drill — reliability per verb over a REAL analytics root.
//!
//! ```text
//! cargo run -p dpl-ledger --example seam_reliability -- <analytics-root>
//! ```

fn main() {
    let root = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: seam_reliability <analytics-root>");
        std::process::exit(2);
    });
    let rows = match dpl_ledger::read_all_rows(std::path::Path::new(&root)) {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let seam_total = rows.iter().filter(|r| r.domain == "seam").count();
    println!(
        "{} rows total, {seam_total} seam rows under {root}",
        rows.len()
    );
    println!(
        "{:<14} {:>9} {:>10} {:>8} {:>12} {:>8}",
        "verb", "executed", "effect_ok", "refused", "effect_rate", "avg_ms"
    );
    for v in dpl_ledger::seam_reliability(&rows) {
        let rate = if v.executed > 0 {
            format!("{:.0}%", 100.0 * v.effect_ok as f64 / v.executed as f64)
        } else {
            "—".into()
        };
        let avg = v
            .total_duration_ms
            .checked_div(v.executed + v.refused)
            .unwrap_or(0);
        println!(
            "{:<14} {:>9} {:>10} {:>8} {:>12} {:>8}",
            v.verb, v.executed, v.effect_ok, v.refused, rate, avg
        );
    }
}
