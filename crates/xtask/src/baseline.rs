//! Fresh remote evidence before work. Never switch, reset, merge or stash.
use std::path::Path;
use std::process::Command;

fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    // spawn-audit: exempt(developer CLI, not launched from production UI)
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|error| format!("baseline: git could not run: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "baseline: git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

pub(super) fn run(args: &[String]) -> Result<(), String> {
    let mode = match args {
        [] => "check",
        [mode] if mode == "check" || mode == "start" => mode,
        _ => return Err("usage: cargo xtask baseline [check|start]".into()),
    };
    let root = std::env::current_dir().map_err(|error| error.to_string())?;
    println!("{}", verify(&root, mode == "start")?);
    Ok(())
}

/// A checkout its owner has retired says so in its own git config
/// (`git config --local dpl.retiredCheckout "<where the work lives now>"`).
/// Unset is the normal case, and `git config --get` then fails.
fn retired(root: &Path) -> Option<String> {
    git(root, &["config", "--local", "--get", "dpl.retiredCheckout"])
        .ok()
        .filter(|note| !note.is_empty())
}

fn verify(root: &Path, starting: bool) -> Result<String, String> {
    if let Some(note) = retired(root) {
        return Err(format!(
            "STOP: this checkout is retired and must not be worked in. {note}"
        ));
    }
    // Explicit refspec, never a cached ref or another task's FETCH_HEAD.
    git(
        root,
        &[
            "fetch",
            "--no-tags",
            "origin",
            "+refs/heads/main:refs/remotes/origin/main",
        ],
    )?;
    let head = git(root, &["rev-parse", "HEAD"])?;
    let main = git(root, &["rev-parse", "refs/remotes/origin/main"])?;
    let hashes = format!("HEAD={head}\norigin/main={main}");
    let base = git(root, &["merge-base", "HEAD", "refs/remotes/origin/main"])?;
    if base != main {
        return Err(format!("{hashes}\nSTOP: behind or diverged from freshly fetched main. Preserve existing work and reconcile before editing. Do not reset it."));
    }
    if starting && (head != main || !git(root, &["status", "--porcelain"])?.is_empty()) {
        return Err(format!("{hashes}\nSTOP: finish and merge the existing milestone, then fast-forward main and leave a clean checkout before starting new work."));
    }
    Ok(format!(
        "{hashes}\nbaseline {} passed",
        if starting { "start" } else { "check" }
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    struct Repo {
        root: PathBuf,
        work: PathBuf,
        upstream: PathBuf,
    }
    impl Repo {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "dpl-baseline-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir_all(&root).unwrap();
            let upstream = root.join("origin.git");
            git(
                &root,
                &[
                    "init",
                    "--bare",
                    "--initial-branch=main",
                    upstream.to_str().unwrap(),
                ],
            )
            .unwrap();
            git(&root, &["clone", upstream.to_str().unwrap(), "work"]).unwrap();
            let work = root.join("work");
            for (key, value) in [
                ("user.name", "Baseline test"),
                ("user.email", "baseline@example.invalid"),
                ("commit.gpgsign", "false"),
                ("core.hooksPath", ".no-hooks"),
            ] {
                git(&work, &["config", key, value]).unwrap();
            }
            let repo = Self {
                root,
                work,
                upstream,
            };
            repo.commit("initial");
            git(&repo.work, &["push", "origin", "HEAD:main"]).unwrap();
            repo
        }
        fn commit(&self, name: &str) {
            git(&self.work, &["commit", "--allow-empty", "-m", name]).unwrap();
        }
    }
    impl Drop for Repo {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }
    #[test]
    fn current_start_passes_but_pending_work_must_land_first() {
        let repo = Repo::new();
        assert!(verify(&repo.work, true).is_ok());
        std::fs::write(repo.work.join("draft"), "preserve me").unwrap();
        assert!(verify(&repo.work, true)
            .unwrap_err()
            .contains("finish and merge"));
        assert!(verify(&repo.work, false).is_ok());
        repo.commit("local work");
        assert!(verify(&repo.work, true).is_err());
        assert!(verify(&repo.work, false).is_ok());
        assert_eq!(
            std::fs::read_to_string(repo.work.join("draft")).unwrap(),
            "preserve me"
        );
    }
    #[test]
    fn fetch_discovers_stale_and_divergent_heads_without_changing_work() {
        let repo = Repo::new();
        let original = git(&repo.work, &["rev-parse", "HEAD"]).unwrap();
        repo.commit("upstream advance");
        git(&repo.work, &["push", "origin", "HEAD:main"]).unwrap();
        git(&repo.work, &["switch", "--detach", &original]).unwrap();
        assert!(verify(&repo.work, false)
            .unwrap_err()
            .contains("behind or diverged"));
        assert_eq!(git(&repo.work, &["rev-parse", "HEAD"]).unwrap(), original);
        repo.commit("diverged");
        assert!(verify(&repo.work, false)
            .unwrap_err()
            .contains("behind or diverged"));
    }
    #[test]
    fn a_retired_checkout_is_refused_before_anything_else() {
        let repo = Repo::new();
        assert!(verify(&repo.work, false).is_ok());
        git(
            &repo.work,
            &[
                "config",
                "--local",
                "dpl.retiredCheckout",
                "Work in THE checkout.",
            ],
        )
        .unwrap();
        for starting in [false, true] {
            let refusal = verify(&repo.work, starting).unwrap_err();
            assert!(refusal.contains("retired"));
            assert!(refusal.contains("Work in THE checkout."));
        }
    }
    #[test]
    fn failed_fetch_never_accepts_a_cached_main() {
        let repo = Repo::new();
        assert!(verify(&repo.work, false).is_ok());
        git(
            &repo.work,
            &[
                "remote",
                "set-url",
                "origin",
                repo.upstream.join("missing").to_str().unwrap(),
            ],
        )
        .unwrap();
        assert!(verify(&repo.work, false).unwrap_err().contains("fetch"));
    }
}
