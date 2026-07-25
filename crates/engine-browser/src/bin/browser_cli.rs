//! Manual proof CLI for the headless-browser driver (DEMO-B):
//!
//! ```text
//! browser_cli shot <url> <out.png> [--window WxH]
//! ```
//!
//! Resolves the installed browser, launches headless, navigates, pins the
//! viewport, screenshots, closes. Exit 0 only when the PNG landed.

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

fn run(args: &[String]) -> Result<(), String> {
    const USAGE: &str = "usage: browser_cli shot <url> <out.png> [--window WxH]";
    match args.first().map(String::as_str) {
        Some("shot") => {
            let url = args.get(1).ok_or(USAGE)?;
            let out = std::path::PathBuf::from(args.get(2).ok_or(USAGE)?);
            let window = match args.get(3).map(String::as_str) {
                Some("--window") => parse_window(args.get(4).ok_or(USAGE)?)?,
                Some(other) => return Err(format!("unknown flag {other:?}\n{USAGE}")),
                None => BrowserConfig::default().window,
            };
            let mut browser = Browser::launch(&BrowserConfig { window })?;
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
        _ => Err(USAGE.into()),
    }
}
