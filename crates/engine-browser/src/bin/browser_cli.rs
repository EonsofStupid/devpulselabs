//! Manual proof + operator CLI for the headless-browser driver (DEMO-B):
//!
//! ```text
//! browser_cli shot <url> <out.png> [--window WxH] [--profile <dir>]
//! browser_cli login <profile-dir> [start-url]
//! ```
//!
//! `shot`: resolve the installed browser, launch (headless), navigate,
//! pin the viewport, screenshot, close. Exit 0 only when the PNG landed.
//!
//! `login` (DG-B.1, the sign-in ceremony): open a VISIBLE window on a
//! persistent named profile — the operator signs in (Google, Twilio, …)
//! and closes the browser; the sessions persist in the profile, so later
//! `--profile` captures ride them. Credentials are typed by the operator
//! only, never by tooling; the profile is the app's own, never the
//! operator's real browser profile (ADR-013 §5).

use std::path::PathBuf;
use std::process::ExitCode;

use dpl_engine_browser::{Browser, BrowserConfig};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("browser_cli: {e}");
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

const USAGE: &str = "usage: browser_cli shot <url> <out.png> [--window WxH] [--profile <dir>]\n       browser_cli login <profile-dir> [start-url]";

fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("shot") => {
            let url = args.get(1).ok_or(USAGE)?;
            let out = PathBuf::from(args.get(2).ok_or(USAGE)?);
            let mut window = BrowserConfig::default().window;
            let mut profile = None;
            let mut rest = args[3..].iter();
            while let Some(flag) = rest.next() {
                match flag.as_str() {
                    "--window" => window = parse_window(rest.next().ok_or(USAGE)?)?,
                    "--profile" => profile = Some(PathBuf::from(rest.next().ok_or(USAGE)?)),
                    other => return Err(format!("unknown flag {other:?}\n{USAGE}")),
                }
            }
            let mut browser = Browser::launch(&BrowserConfig {
                window,
                profile,
                headless: true,
            })?;
            let (bin, source) = browser.binary();
            eprintln!("browser_cli: driving {} ({source})", bin.display());
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
            let profile = PathBuf::from(args.get(1).ok_or(USAGE)?);
            let url = args
                .get(2)
                .map(String::as_str)
                .unwrap_or("https://accounts.google.com");
            let mut browser = Browser::launch(&BrowserConfig {
                window: (1280, 900),
                profile: Some(profile.clone()),
                headless: false,
            })?;
            let (bin, source) = browser.binary();
            eprintln!("browser_cli: opened {} ({source})", bin.display());
            {
                let mut page = browser.new_page()?;
                page.navigate(url)?;
            }
            eprintln!(
                "browser_cli: sign in in the window, then CLOSE the browser \
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
