//! Dump every ledger row under an analytics root — the eyeball drill.
//!
//! ```text
//! cargo run -p dpl-ledger --example dump_rows -- <analytics-root> [domain]
//! ```

fn main() {
    let mut args = std::env::args().skip(1);
    let root = args.next().unwrap_or_else(|| {
        eprintln!("usage: dump_rows <analytics-root> [domain]");
        std::process::exit(2);
    });
    let domain = args.next();
    let rows = match dpl_ledger::read_all_rows(std::path::Path::new(&root)) {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    for r in rows
        .iter()
        .filter(|r| domain.as_deref().is_none_or(|d| r.domain == d))
    {
        println!(
            "seq={} ms={} {}/{} {}",
            r.seq, r.ms, r.domain, r.type_name, r.body
        );
    }
}
