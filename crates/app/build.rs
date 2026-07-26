// spawn-audit: exempt(build script — compile-time git stamp, no GUI)
//! Stamp the build so the RUNNING APP proves which code it is — the footer
//! shows `<short-hash> · <utc time>`. Kills the "am I looking at a stale
//! exe" failure mode for good.

use std::process::Command;

fn main() {
    // Rebuild the stamp whenever HEAD moves.
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/refs/heads");

    let hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "nogit".into());

    // Seconds since epoch → a compact UTC stamp without a chrono dep.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (days, rem) = (secs / 86_400, secs % 86_400);
    // Civil date from day count (Howard Hinnant's algorithm, verbatim).
    let (y, m, d) = {
        let z = days as i64 + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z.rem_euclid(146_097);
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = doy - (153 * mp + 2) / 5 + 1;
        let month = if mp < 10 { mp + 3 } else { mp - 9 };
        (era * 400 + yoe + i64::from(month <= 2), month, day)
    };
    let stamp = format!(
        "{hash} · {y:04}-{m:02}-{d:02} {:02}:{:02}z",
        rem / 3600,
        (rem % 3600) / 60
    );

    println!("cargo:rustc-env=DPL_BUILD={stamp}");
}
