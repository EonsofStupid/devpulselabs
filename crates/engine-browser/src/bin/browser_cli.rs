//! Manual proof + operator CLI for the headless-browser driver (DEMO-B):
//!
//! ```text
//! browser-cli shot <url> [out.png] [--job <job>] [--name <stem>]
//!                        [--window WxH] [--profile <name|dir>]
//! browser-cli login <profile-name|dir> [start-url]
//! ```
//!
//! The frictionless law (ADR-014): with no explicit `out.png`, a shot
//! auto-files into `assets\captures\web\<job>\<stem>.png` (no-clobber) —
//! generated content always lands inside the app. Explicit paths remain
//! for harness use.
//!
//! `login` (DG-B.1): a VISIBLE window on a persistent profile — a bare
//! name lives on the app's shelf (`.userdata\profiles\<name>`). The
//! operator signs in and closes the browser; sessions persist. The
//! operator types credentials, never tooling.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use dpl_engine_browser::{profile_dir, Browser, BrowserConfig};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("browser-cli: {e}");
            ExitCode::FAILURE
        }
    }
}

fn parse_window(s: &str) -> Result<(u32, u32), String> {
    let (w, h) = s
        .split_once('x')
        .ok_or_else(|| format!("--window wants WxH, got {s:?}"))?;
    Ok((
        w.parse().map_err(|e| format!("--window width: {e}"))?,
        h.parse().map_err(|e| format!("--window height: {e}"))?,
    ))
}

/// The first free `<stem>.png` / `<stem>-N.png` inside `dir`.
fn no_clobber_png(dir: &Path, stem: &str) -> PathBuf {
    let first = dir.join(format!("{stem}.png"));
    if !first.exists() {
        return first;
    }
    (2..)
        .map(|n| dir.join(format!("{stem}-{n}.png")))
        .find(|p| !p.exists())
        .expect("unbounded range")
}

const USAGE: &str = "usage: browser-cli shot <url> [out.png] [--job <job>] [--name <stem>] \
                     [--window WxH] [--profile <name|dir>]\n       \
                     browser-cli login <profile-name|dir> [start-url]";

fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("shot") => {
            let url = args.get(1).ok_or(USAGE)?;
            let mut explicit_out: Option<PathBuf> = None;
            let mut job: Option<&str> = None;
            let mut name: Option<&str> = None;
            let mut window = BrowserConfig::default().window;
            let mut profile = None;
            let mut rest = args[2..].iter();
            while let Some(a) = rest.next() {
                match a.as_str() {
                    "--job" => job = Some(rest.next().ok_or(USAGE)?),
                    "--name" => name = Some(rest.next().ok_or(USAGE)?),
                    "--window" => window = parse_window(rest.next().ok_or(USAGE)?)?,
                    "--profile" => profile = Some(profile_dir(rest.next().ok_or(USAGE)?)),
                    flag if flag.starts_with("--") => {
                        return Err(format!("unknown flag {flag:?}\n{USAGE}"))
                    }
                    positional if explicit_out.is_none() => {
                        explicit_out = Some(PathBuf::from(positional));
                    }
                    extra => return Err(format!("unexpected argument {extra:?}\n{USAGE}")),
                }
            }
            let out = match (explicit_out, job) {
                (Some(p), _) => p,
                (None, Some(job)) => {
                    let dir = dpl_engine_capture::assets_capture_dir("web", job)?;
                    no_clobber_png(&dir, name.unwrap_or("shot"))
                }
                (None, None) => {
                    return Err(format!(
                        "a shot needs a destination: pass out.png, or --job <job> \
                         to auto-file under assets\\captures\\web\\\n{USAGE}"
                    ))
                }
            };
            let mut browser = Browser::launch(&BrowserConfig {
                window,
                executable: None,
                profile,
                headless: true,
            })?;
            let (bin, source) = browser.binary();
            eprintln!("browser-cli: driving {} ({source})", bin.display());
            let bytes = {
                let mut page = browser.new_page()?;
                page.set_viewport(window.0, window.1)?;
                page.navigate(url)?;
                page.screenshot_png(&out)?
            };
            browser.close()?;
            println!("{} ({bytes} bytes)", out.display());
            Ok(())
        }
        Some("login") => {
            let profile = profile_dir(args.get(1).ok_or(USAGE)?);
            let url = args
                .get(2)
                .map(String::as_str)
                .unwrap_or("https://accounts.google.com");
            let mut browser = Browser::launch(&BrowserConfig {
                window: (1280, 900),
                executable: None,
                profile: Some(profile.clone()),
                headless: false,
            })?;
            let (bin, source) = browser.binary();
            eprintln!("browser-cli: opened {} ({source})", bin.display());
            {
                let mut page = browser.new_page()?;
                page.navigate(url)?;
            }
            eprintln!(
                "browser-cli: sign in in the window, then CLOSE the browser \
                 to finish. Sessions persist in {}",
                profile.display()
            );
            browser.wait_until_closed()?;
            println!("profile saved: {}", profile.display());
            Ok(())
        }
        _ => Err(USAGE.into()),
    }
}
