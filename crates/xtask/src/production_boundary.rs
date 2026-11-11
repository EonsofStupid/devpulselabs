//! Exercise Cargo's resolved production graph, including platform-specific edges.
// spawn-audit: exempt(build-tool dependency inspection; no product process)
use std::process::{Command, Stdio};

pub fn audit() -> Result<(), String> {
    let output = Command::new("cargo")
        .args([
            "tree",
            "--locked",
            "-p",
            "devpulselabs",
            "--no-default-features",
            "--edges",
            "normal",
            "--target",
            "all",
            "--prefix",
            "none",
            "--format",
            "{p}",
        ])
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| format!("production boundary: cargo tree failed: {e}"))?;
    if !output.status.success() {
        return Err(format!("production boundary: cargo tree {}", output.status));
    }
    let graph = String::from_utf8(output.stdout)
        .map_err(|e| format!("production boundary: invalid Cargo output: {e}"))?;
    let forbidden = [
        "dpl-coach-inventory",
        "dpl-coach-fabric",
        "dpl-coach-runtime",
        "dpl-coach-overlay",
        "dpl-coach-daemon",
        "egui-pilot-ai-runtime",
        "egui-pilot-ai-transport",
    ];
    let leaked: Vec<_> = graph
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .filter(|name| forbidden.contains(name))
        .collect();
    if !leaked.is_empty() {
        return Err(format!(
            "production boundary: devtools packages in normal app graph: {leaked:?}"
        ));
    }
    if !graph.lines().any(|line| line.starts_with("devpulselabs ")) {
        return Err("production boundary: app missing from Cargo graph".into());
    }
    println!("production boundary: normal app excludes foreign Coach and control runtime");
    Ok(())
}
