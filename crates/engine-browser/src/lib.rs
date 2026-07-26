//! Headless Chromium orchestration (DEMO-B) — the Program DEMO external-
//! tier driver.
//!
//! The browser is a SUBPROCESS, never a crate dep (ADR-013 — the pattern
//! ffmpeg and aerender already follow). We RESOLVE an installed
//! chromium-family binary (injected path → `DPL_BROWSER` → bundled →
//! Chrome installs → Edge, the Windows zero-install fallback → PATH),
//! launch it `--headless=new` with a FRESH ephemeral profile and
//! `--remote-debugging-port=0`, read the real port from the
//! `DevToolsActivePort` file (never guess, never race), and speak the
//! DevTools protocol over a localhost WebSocket: id-matched
//! request/response, events drained to a queue. Sync and threadless —
//! callers run it off the UI thread, like every engine crate.
//!
//! The operator's real browser profile is never touched (ADR-013 §5).

use std::collections::VecDeque;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tungstenite::{Message, WebSocket};

/// One CDP command's reply deadline; also the page-load wait.
const CDP_TIMEOUT: Duration = Duration::from_secs(30);
/// How long the child gets to write `DevToolsActivePort`.
const BOOT_TIMEOUT: Duration = Duration::from_secs(10);

// ───────────────────────── resolve, never download ──────────────────────

static BROWSER: OnceLock<PathBuf> = OnceLock::new();

/// Managed-browser injection — same contract as `set_ffmpeg_path` in the
/// other engines: the embedding shell resolves once and injects.
pub fn set_browser_path(p: PathBuf) {
    let _ = BROWSER.set(p);
}

fn find_on_path(bin: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|d| d.join(bin))
            .find(|p| p.is_file())
    })
}

/// Every place we look, in order, tagged with a source label for honest
/// logs and honest errors.
pub fn candidates() -> Vec<(PathBuf, &'static str)> {
    let mut c: Vec<(PathBuf, &'static str)> = Vec::new();
    if let Some(p) = BROWSER.get() {
        c.push((p.clone(), "injected"));
    }
    if let Some(p) = std::env::var_os("DPL_BROWSER") {
        c.push((PathBuf::from(p), "env DPL_BROWSER"));
    }
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(Path::to_path_buf))
    {
        let bundled = if cfg!(windows) {
            "chrome.exe"
        } else {
            "chrome"
        };
        c.push((dir.join("bin").join(bundled), "bundled"));
    }
    if cfg!(windows) {
        for root in ["LOCALAPPDATA", "ProgramFiles", "ProgramFiles(x86)"] {
            if let Some(base) = std::env::var_os(root) {
                c.push((
                    Path::new(&base)
                        .join("Google")
                        .join("Chrome")
                        .join("Application")
                        .join("chrome.exe"),
                    "chrome-install",
                ));
            }
        }
        // Edge is chromium and ships with Windows — the zero-install
        // fallback that makes the driver work on any operator box.
        for root in ["ProgramFiles(x86)", "ProgramFiles"] {
            if let Some(base) = std::env::var_os(root) {
                c.push((
                    Path::new(&base)
                        .join("Microsoft")
                        .join("Edge")
                        .join("Application")
                        .join("msedge.exe"),
                    "edge-install",
                ));
            }
        }
        for bin in ["chrome.exe", "msedge.exe"] {
            if let Some(p) = find_on_path(bin) {
                c.push((p, "PATH"));
            }
        }
    } else {
        // The Wayland/Ubuntu lane — cross-support is a product
        // requirement, not a nicety.
        for bin in [
            "google-chrome",
            "google-chrome-stable",
            "chromium",
            "chromium-browser",
            "microsoft-edge",
        ] {
            if let Some(p) = find_on_path(bin) {
                c.push((p, "PATH"));
            }
        }
        c.push((PathBuf::from("/snap/bin/chromium"), "snap"));
    }
    c
}

fn missing_error(cands: &[(PathBuf, &'static str)]) -> String {
    let checked: Vec<String> = cands
        .iter()
        .map(|(p, tag)| format!("{} ({tag})", p.display()))
        .collect();
    format!(
        "no chromium-family browser found — checked: {}. Install Chrome \
         or set DPL_BROWSER to a chromium-family binary.",
        checked.join(", ")
    )
}

/// First candidate that exists on disk, with its source tag. A miss names
/// every checked location (the managed-ffmpeg law).
pub fn resolve() -> Result<(PathBuf, &'static str), String> {
    let cands = candidates();
    cands
        .iter()
        .find(|(p, _)| p.is_file())
        .cloned()
        .ok_or_else(|| missing_error(&cands))
}

// ─────────────────────────────── launch ─────────────────────────────────

/// How to launch. Window size is the OS window AND the initial viewport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserConfig {
    pub window: (u32, u32),
    /// `None` → a fresh ephemeral temp profile, removed on close.
    /// `Some(dir)` → a persistent NAMED demo profile (DG-B.1): kept
    /// across runs so an operator-performed sign-in survives — the whole
    /// point. Never the operator's real browser profile (ADR-013 §5).
    pub profile: Option<PathBuf>,
    /// `true` → `--headless=new`. `false` → a real visible window: the
    /// sign-in ceremony, and watch-it-drive demonstrations.
    pub headless: bool,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            window: (1280, 800),
            profile: None,
            headless: true,
        }
    }
}

/// The launch argument VECTOR (never a shell string). Port 0 makes the
/// browser pick a free port and write `DevToolsActivePort` in the profile
/// dir; the isolated profile keeps us out of the operator's real browser.
pub fn launch_args(profile: &Path, window: (u32, u32), headless: bool) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();
    if headless {
        args.push("--headless=new".into());
    }
    args.extend([
        "--remote-debugging-port=0".into(),
        format!("--user-data-dir={}", profile.display()),
        "--no-first-run".into(),
        "--no-default-browser-check".into(),
        "--disable-sync".into(),
        "--disable-extensions".into(),
        format!("--window-size={},{}", window.0, window.1),
        "about:blank".into(),
    ]);
    args
}

/// Parse `DevToolsActivePort`: line 1 the port, line 2 the browser-target
/// ws path (`/devtools/browser/<uuid>`).
pub fn parse_devtools_active_port(s: &str) -> Result<(u16, String), String> {
    let mut lines = s.lines();
    let port = lines
        .next()
        .and_then(|l| l.trim().parse::<u16>().ok())
        .ok_or_else(|| format!("DevToolsActivePort: bad port line in {s:?}"))?;
    let path = lines
        .next()
        .map(str::trim)
        .filter(|p| p.starts_with('/'))
        .ok_or_else(|| format!("DevToolsActivePort: bad ws-path line in {s:?}"))?;
    Ok((port, path.to_string()))
}

static PROFILE_N: AtomicU64 = AtomicU64::new(0);

/// Ephemeral profiles live in Ring 0's regenerable temp (ADR-014) —
/// never the OS temp dir, never outside the app.
fn fresh_profile_dir() -> PathBuf {
    let n = PROFILE_N.fetch_add(1, Ordering::Relaxed);
    dpl_engine_capture::userdata_root()
        .join("temp")
        .join("browser")
        .join(format!("profile-{}-{n}", std::process::id()))
}

/// A bare NAME (no separators) resolves to the app's profile shelf,
/// `.userdata/profiles/<name>`; anything path-like is honored as given.
pub fn profile_dir(name_or_path: &str) -> PathBuf {
    if name_or_path.contains(['/', '\\', ':']) {
        PathBuf::from(name_or_path)
    } else {
        dpl_engine_capture::userdata_root()
            .join("profiles")
            .join(name_or_path)
    }
}

// ─────────────────────────── base64 (CDP payloads) ──────────────────────

fn b64_val(c: u8) -> Option<u32> {
    match c {
        b'A'..=b'Z' => Some(u32::from(c - b'A')),
        b'a'..=b'z' => Some(u32::from(c - b'a') + 26),
        b'0'..=b'9' => Some(u32::from(c - b'0') + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

/// Decode standard base64 (the screenshot payload). Strict alphabet;
/// decoding stops at `=` padding. ~25 lines beats a dependency.
pub fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;
    for &c in s.as_bytes() {
        if c == b'=' {
            break;
        }
        let v = b64_val(c).ok_or_else(|| format!("base64: unexpected byte {c:#04x}"))?;
        buf = (buf << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    Ok(out)
}

// ───────────────────────────── the CDP client ───────────────────────────

/// A running headless browser: the child process, its ephemeral profile,
/// and the browser-target CDP socket. Dropping it kills the child and
/// removes the profile — a driver never leaks browsers.
pub struct Browser {
    child: Child,
    profile: PathBuf,
    /// Ephemeral profiles are removed on close/drop; persistent DG-B.1
    /// profiles (operator sign-ins live there) are NEVER touched.
    ephemeral: bool,
    binary: (PathBuf, &'static str),
    ws: WebSocket<TcpStream>,
    next_id: u64,
    events: VecDeque<Value>,
}

impl Browser {
    /// Resolve, launch, wait for `DevToolsActivePort`, connect.
    pub fn launch(cfg: &BrowserConfig) -> Result<Self, String> {
        let (bin, source) = resolve()?;
        let ephemeral = cfg.profile.is_none();
        let profile = cfg.profile.clone().unwrap_or_else(fresh_profile_dir);
        std::fs::create_dir_all(&profile)
            .map_err(|e| format!("profile dir {}: {e}", profile.display()))?;
        // A persistent profile carries last run's DevToolsActivePort;
        // remove it so we only ever read the file THIS child writes.
        let _ = std::fs::remove_file(profile.join("DevToolsActivePort"));
        let mut cmd = Command::new(&bin);
        cmd.args(launch_args(&profile, cfg.window, cfg.headless))
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let mut child = dpl_proc::quiet(&mut cmd)
            .spawn()
            .map_err(|e| format!("spawn {}: {e}", bin.display()))?;

        let port_file = profile.join("DevToolsActivePort");
        let deadline = Instant::now() + BOOT_TIMEOUT;
        let (port, ws_path) = loop {
            if let Ok(s) = std::fs::read_to_string(&port_file) {
                if let Ok(v) = parse_devtools_active_port(&s) {
                    break v;
                }
            }
            if let Ok(Some(status)) = child.try_wait() {
                if ephemeral {
                    let _ = std::fs::remove_dir_all(&profile);
                }
                return Err(format!(
                    "browser exited during boot ({status}) — if this is a \
                     persistent profile, a browser may already be running on it"
                ));
            }
            if Instant::now() > deadline {
                let _ = child.kill();
                let _ = child.wait();
                if ephemeral {
                    let _ = std::fs::remove_dir_all(&profile);
                }
                return Err(format!(
                    "browser never wrote {} within {BOOT_TIMEOUT:?}",
                    port_file.display()
                ));
            }
            std::thread::sleep(Duration::from_millis(50));
        };

        let stream = TcpStream::connect(("127.0.0.1", port))
            .map_err(|e| format!("connect 127.0.0.1:{port}: {e}"))?;
        stream.set_nodelay(true).ok();
        let url = format!("ws://127.0.0.1:{port}{ws_path}");
        // Handshake on a blocking socket; the read timeout goes on AFTER
        // (a timeout mid-handshake would surface as a spurious failure).
        let (ws, _resp) =
            tungstenite::client(url.as_str(), stream).map_err(|e| format!("cdp handshake: {e}"))?;
        ws.get_ref()
            .set_read_timeout(Some(Duration::from_millis(200)))
            .map_err(|e| format!("socket timeout: {e}"))?;
        Ok(Self {
            child,
            profile,
            ephemeral,
            binary: (bin, source),
            ws,
            next_id: 0,
            events: VecDeque::new(),
        })
    }

    /// Hand the window to the operator and block until THEY close the
    /// browser — the DG-B.1 sign-in ceremony. Whatever sessions they
    /// signed into persist in the (named) profile for later captures.
    pub fn wait_until_closed(mut self) -> Result<(), String> {
        self.child
            .wait()
            .map(|_| ())
            .map_err(|e| format!("waiting for browser close: {e}"))
    }

    /// Which binary is actually driving, and where it came from.
    pub fn binary(&self) -> (&Path, &'static str) {
        (&self.binary.0, self.binary.1)
    }

    /// Read one CDP JSON message; `None` once `deadline` passes.
    /// Socket-level timeouts are polls, not failures.
    fn read_one(&mut self, deadline: Instant) -> Result<Option<Value>, String> {
        loop {
            if Instant::now() > deadline {
                return Ok(None);
            }
            match self.ws.read() {
                Ok(Message::Text(t)) => {
                    return serde_json::from_str(t.as_str())
                        .map(Some)
                        .map_err(|e| format!("cdp json: {e}"))
                }
                // Binary/ping/pong frames are not CDP traffic.
                Ok(_) => {}
                Err(tungstenite::Error::Io(e))
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) => {}
                Err(e) => return Err(format!("cdp read: {e}")),
            }
        }
    }

    /// One CDP command: send, then drain messages (stashing events) until
    /// the id-matched reply arrives. A protocol `error` member is an Err.
    fn call(
        &mut self,
        session: Option<&str>,
        method: &str,
        params: Value,
    ) -> Result<Value, String> {
        self.next_id += 1;
        let id = self.next_id;
        let mut msg = json!({ "id": id, "method": method, "params": params });
        if let Some(s) = session {
            msg["sessionId"] = json!(s);
        }
        self.ws
            .send(Message::text(msg.to_string()))
            .map_err(|e| format!("cdp send {method}: {e}"))?;
        let deadline = Instant::now() + CDP_TIMEOUT;
        loop {
            match self.read_one(deadline)? {
                None => return Err(format!("cdp {method}: no reply within {CDP_TIMEOUT:?}")),
                Some(v) => {
                    if v.get("id").and_then(Value::as_u64) == Some(id) {
                        if let Some(err) = v.get("error") {
                            return Err(format!("cdp {method}: {err}"));
                        }
                        return Ok(v.get("result").cloned().unwrap_or(Value::Null));
                    }
                    if v.get("method").is_some() {
                        self.events.push_back(v);
                    }
                }
            }
        }
    }

    /// Wait for a session-scoped event, draining and stashing others.
    fn wait_event(
        &mut self,
        session: &str,
        method: &str,
        timeout: Duration,
    ) -> Result<Value, String> {
        let matches_wanted = |v: &Value| {
            v.get("method").and_then(Value::as_str) == Some(method)
                && v.get("sessionId").and_then(Value::as_str) == Some(session)
        };
        if let Some(pos) = self.events.iter().position(matches_wanted) {
            return Ok(self.events.remove(pos).expect("position exists"));
        }
        let deadline = Instant::now() + timeout;
        loop {
            match self.read_one(deadline)? {
                None => return Err(format!("timed out waiting for {method} ({timeout:?})")),
                Some(v) => {
                    if matches_wanted(&v) {
                        return Ok(v);
                    }
                    if v.get("method").is_some() {
                        self.events.push_back(v);
                    }
                }
            }
        }
    }

    /// Open a fresh page target and attach a flat session to it. One page
    /// at a time in this pass — the driver's demos are linear scripts.
    pub fn new_page(&mut self) -> Result<Page<'_>, String> {
        let r = self.call(None, "Target.createTarget", json!({"url": "about:blank"}))?;
        let target_id = r
            .get("targetId")
            .and_then(Value::as_str)
            .ok_or("createTarget: no targetId")?
            .to_string();
        let r = self.call(
            None,
            "Target.attachToTarget",
            json!({"targetId": target_id, "flatten": true}),
        )?;
        let session = r
            .get("sessionId")
            .and_then(Value::as_str)
            .ok_or("attachToTarget: no sessionId")?
            .to_string();
        self.call(Some(&session), "Page.enable", json!({}))?;
        self.call(Some(&session), "Runtime.enable", json!({}))?;
        Ok(Page {
            browser: self,
            session,
            target_id,
        })
    }

    /// Graceful close: `Browser.close`, ≤5 s to exit, then kill. The
    /// ephemeral profile is removed best-effort.
    pub fn close(mut self) -> Result<(), String> {
        let _ = self.call(None, "Browser.close", json!({}));
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match self.child.try_wait() {
                Ok(Some(_)) | Err(_) => break,
                Ok(None) => {
                    if Instant::now() > deadline {
                        let _ = self.child.kill();
                        let _ = self.child.wait();
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
        }
        Ok(())
    }
}

impl Drop for Browser {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if self.ephemeral {
            let _ = std::fs::remove_dir_all(&self.profile);
        }
    }
}

/// One attached page. Borrows the browser mutably — the driver runs one
/// linear script per page, which is exactly what a demo capture is.
pub struct Page<'b> {
    browser: &'b mut Browser,
    session: String,
    pub target_id: String,
}

impl Page<'_> {
    /// Navigate and wait for `Page.loadEventFired`.
    pub fn navigate(&mut self, url: &str) -> Result<(), String> {
        let r = self
            .browser
            .call(Some(&self.session), "Page.navigate", json!({"url": url}))?;
        if let Some(err) = r.get("errorText").and_then(Value::as_str) {
            if !err.is_empty() {
                return Err(format!("navigate {url}: {err}"));
            }
        }
        self.browser
            .wait_event(&self.session, "Page.loadEventFired", CDP_TIMEOUT)?;
        Ok(())
    }

    /// Evaluate an expression, awaiting promises, returning it by value.
    pub fn evaluate(&mut self, expr: &str) -> Result<Value, String> {
        let r = self.browser.call(
            Some(&self.session),
            "Runtime.evaluate",
            json!({"expression": expr, "returnByValue": true, "awaitPromise": true}),
        )?;
        if let Some(ex) = r.get("exceptionDetails") {
            let text = ex
                .pointer("/exception/description")
                .or_else(|| ex.get("text"))
                .and_then(Value::as_str)
                .unwrap_or("unknown exception");
            return Err(format!("evaluate: {text}"));
        }
        Ok(r.pointer("/result/value").cloned().unwrap_or(Value::Null))
    }

    /// Pin the viewport (device metrics) — REAL sizes, the proof-shot law.
    pub fn set_viewport(&mut self, w: u32, h: u32) -> Result<(), String> {
        self.browser
            .call(
                Some(&self.session),
                "Emulation.setDeviceMetricsOverride",
                json!({"width": w, "height": h, "deviceScaleFactor": 1, "mobile": false}),
            )
            .map(|_| ())
    }

    /// PNG screenshot to `out` (parent created). Returns the byte count.
    pub fn screenshot_png(&mut self, out: &Path) -> Result<usize, String> {
        let r = self.browser.call(
            Some(&self.session),
            "Page.captureScreenshot",
            json!({"format": "png"}),
        )?;
        let data = r
            .get("data")
            .and_then(Value::as_str)
            .ok_or("captureScreenshot: no data")?;
        let bytes = base64_decode(data)?;
        if let Some(dir) = out.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("shot dir: {e}"))?;
        }
        std::fs::write(out, &bytes).map_err(|e| format!("shot write {}: {e}", out.display()))?;
        Ok(bytes.len())
    }

    /// A left click at viewport coordinates (press + release).
    pub fn click(&mut self, x: f64, y: f64) -> Result<(), String> {
        for kind in ["mousePressed", "mouseReleased"] {
            self.browser.call(
                Some(&self.session),
                "Input.dispatchMouseEvent",
                json!({"type": kind, "x": x, "y": y, "button": "left", "clickCount": 1}),
            )?;
        }
        Ok(())
    }

    /// Insert text into the focused element (the layman typing step).
    pub fn insert_text(&mut self, text: &str) -> Result<(), String> {
        self.browser
            .call(
                Some(&self.session),
                "Input.insertText",
                json!({"text": text}),
            )
            .map(|_| ())
    }
}

// ────────────────────────────────── tests ───────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_args_are_headless_portless_and_profiled() {
        let a = launch_args(Path::new("C:/tmp/prof"), (1366, 768), true).join(" ");
        assert!(a.contains("--headless=new"), "{a}");
        assert!(a.contains("--remote-debugging-port=0"), "{a}");
        assert!(a.contains("--user-data-dir=C:/tmp/prof"), "{a}");
        assert!(a.contains("--no-first-run"), "{a}");
        assert!(a.contains("--window-size=1366,768"), "{a}");
        assert!(a.ends_with("about:blank"), "{a}");
    }

    #[test]
    fn headful_launch_keeps_everything_but_the_headless_flag() {
        let a = launch_args(Path::new("/p"), (1280, 900), false).join(" ");
        assert!(!a.contains("--headless"), "{a}");
        assert!(a.contains("--remote-debugging-port=0"), "{a}");
        assert!(a.contains("--user-data-dir=/p"), "{a}");
    }

    #[test]
    fn devtools_active_port_parses_and_rejects() {
        let (port, path) =
            parse_devtools_active_port("54321\n/devtools/browser/abc-123\n").unwrap();
        assert_eq!(port, 54321);
        assert_eq!(path, "/devtools/browser/abc-123");
        assert!(parse_devtools_active_port("").is_err());
        assert!(parse_devtools_active_port("notaport\n/x").is_err());
        assert!(parse_devtools_active_port("80\nno-leading-slash").is_err());
    }

    #[test]
    fn base64_round_trips_the_alphabet() {
        assert_eq!(base64_decode("aGVsbG8=").unwrap(), b"hello");
        assert_eq!(base64_decode("aGVsbG8h").unwrap(), b"hello!");
        assert_eq!(base64_decode("aA==").unwrap(), b"h");
        assert_eq!(base64_decode("").unwrap(), b"");
        // PNG magic survives: iVBORw0KGgo= is the classic PNG prefix.
        assert_eq!(
            base64_decode("iVBORw0KGgo=").unwrap()[..4],
            [0x89, b'P', b'N', b'G']
        );
        assert!(base64_decode("he!lo").is_err());
    }

    #[test]
    fn profile_names_resolve_to_the_userdata_shelf_paths_pass_through() {
        let named = profile_dir("clyffy");
        assert!(named.ends_with("profiles/clyffy") || named.ends_with("profiles\\clyffy"));
        assert_eq!(profile_dir("C:/x/y"), PathBuf::from("C:/x/y"));
        assert_eq!(profile_dir("./rel/dir"), PathBuf::from("./rel/dir"));
    }

    #[test]
    fn env_override_outranks_installs_in_candidate_order() {
        std::env::set_var("DPL_BROWSER", "Z:/definitely/custom/browser.exe");
        let c = candidates();
        let env_pos = c
            .iter()
            .position(|(_, tag)| *tag == "env DPL_BROWSER")
            .expect("env candidate present");
        let first_install = c
            .iter()
            .position(|(_, tag)| matches!(*tag, "chrome-install" | "edge-install" | "PATH"));
        if let Some(install_pos) = first_install {
            assert!(env_pos < install_pos, "env override must outrank installs");
        }
        std::env::remove_var("DPL_BROWSER");
    }

    #[test]
    fn a_miss_names_every_checked_location() {
        let cands = vec![
            (PathBuf::from("C:/nope/chrome.exe"), "chrome-install"),
            (PathBuf::from("C:/nada/msedge.exe"), "edge-install"),
        ];
        let e = missing_error(&cands);
        assert!(e.contains("C:/nope/chrome.exe (chrome-install)"), "{e}");
        assert!(e.contains("C:/nada/msedge.exe (edge-install)"), "{e}");
        assert!(e.contains("DPL_BROWSER"), "{e}");
    }
}
