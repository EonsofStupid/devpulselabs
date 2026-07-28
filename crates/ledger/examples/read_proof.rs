fn main() {
    let path = std::env::args().nth(1).expect("path");
    let rows = dpl_ledger::read_parquet_rows(std::path::Path::new(&path)).unwrap();
    println!("rows={}", rows.len());
    for r in &rows {
        println!(
            "{} seq={} type={} body={}",
            r.domain, r.seq, r.type_name, r.body
        );
    }
    assert!(
        rows.iter().any(|r| {
            r.domain == "intent"
                && r.type_name == "determinismChanged"
                && r.body.contains("evidence")
        }),
        "missing evidence intent row: {rows:?}"
    );
    println!("PROOF OK");
}
