//! dpl-guide — the `.guide` CONTENT MODEL + the standalone renderers (W5).
//!
//! A guide export is a FOLDER artifact, complete in itself — the forward
//! payload of the ratified native Guide Player's `.guide` ZIP:
//!
//! ```text
//! inbox/<name>/
//!   guide.json        the export manifest (this crate's Manifest)
//!   step-001.webp …   the KEPT stills, renumbered (do + optional done)
//!   session.mp4       the light replay re-encode (when a session ships)
//!   guide.html        ONE self-contained double-click file, phone-loadable
//!   guide-video.mp4   the single standalone video render (optional)
//! ```
//!
//! Laws: sources are never mutated (everything here writes NEW files);
//! `guide.html` makes ZERO external requests (stills — and by default the
//! replay video — ride as data URIs; the sidecar mode references only
//! sibling files); ffmpeg legs go through `dpl-engine-media` (ADR-009:
//! subprocess, never a crate dep); base64 is hand-rolled — no new deps.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// One step of the guide — the Steps recorder's `StepRec` shape plus the
/// reviewer's `caption`. The window facts (app/title/width/height/bytes)
/// are carried so a review round-trip through `engine-steps::edit` stays
/// lossless.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    pub order: u32,
    pub file: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_file: Option<String>,
    #[serde(default)]
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub click: Option<[f64; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    #[serde(default)]
    pub app: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
    #[serde(default)]
    pub bytes: usize,
}

/// One replay chapter tick — a recorded fabric event at `ms`.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Tick {
    pub ms: u64,
    pub label: String,
}

/// The bundled session replay: the (light, re-encoded) video file's name
/// beside the manifest, plus its event chapters.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct SessionRef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video: Option<String>,
    #[serde(default)]
    pub events: Vec<Tick>,
}

/// The `.guide` manifest — loads the Steps recorder's `guide.json`
/// unchanged (unknown recorder fields tolerated) and is what an export
/// writes back beside its assets.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub kind: String,
    pub name: String,
    pub step_count: usize,
    pub steps: Vec<Step>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guide_video: Option<String>,
    /// The social clip beside the manifest (W5.3) — the replay as an
    /// animated GIF for drop-anywhere embeds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replay_gif: Option<String>,
}

/// Export knobs — the operator-ratified defaults: replay EMBEDDED in the
/// html (one truly self-reliant file), the standalone video rendered too.
#[derive(Clone, Copy, Debug)]
pub struct ExportOpts {
    pub embed_video: bool,
    pub make_guide_video: bool,
    /// Also render the replay as a social GIF (`replay.gif`).
    pub make_gif: bool,
    pub secs_per_still: f32,
}

impl Default for ExportOpts {
    fn default() -> Self {
        Self {
            embed_video: true,
            make_guide_video: true,
            make_gif: true,
            secs_per_still: 3.0,
        }
    }
}

/// What an export produced (paths that exist when the call returns).
#[derive(Debug)]
pub struct Export {
    pub dir: PathBuf,
    pub html: PathBuf,
    pub video: Option<PathBuf>,
    pub steps: usize,
}

/// Load a guide folder's `guide.json` (recorder-written or exported).
pub fn load_guide(dir: &Path) -> Result<Manifest, String> {
    let path = dir.join("guide.json");
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("not a guide folder ({}): {e}", path.display()))?;
    serde_json::from_str(&raw).map_err(|e| format!("parse {}: {e}", path.display()))
}

/// Build a `SessionRef` from a session FOLDER (`session.mp4` +
/// `events.jsonl`). Absent files degrade honestly — a partial session
/// still yields its event track.
pub fn load_session(dir: &Path) -> SessionRef {
    SessionRef {
        video: dir
            .join("session.mp4")
            .is_file()
            .then(|| "session.mp4".to_string()),
        events: parse_events(&dir.join("events.jsonl")),
    }
}

/// events.jsonl → chapter ticks. Each line parses independently; anything
/// malformed is skipped, never invented (the S1 player's exact mapping).
pub fn parse_events(path: &Path) -> Vec<Tick> {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    raw.lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .filter_map(|v| {
            let ms = v.get("ms")?.as_u64()?;
            let kind = v.get("type")?.as_str()?.to_string();
            let label = match kind.as_str() {
                "route-changed" => format!(
                    "{}/{}",
                    v.get("hub").and_then(|h| h.as_str()).unwrap_or("?"),
                    v.get("sub").and_then(|s| s.as_str()).unwrap_or("?"),
                ),
                "full-bleed" => {
                    if v.get("on").and_then(|o| o.as_bool()).unwrap_or(false) {
                        "full-bleed on".into()
                    } else {
                        "full-bleed off".into()
                    }
                }
                other => other.to_string(),
            };
            Some(Tick { ms, label })
        })
        .collect()
}

/// RFC 4648 base64 (standard alphabet, padded) — hand-rolled so the
/// export adds zero deps.
pub fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        let n = u32::from(chunk[0]) << 16 | u32::from(b1) << 8 | u32::from(b2);
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            T[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// A bare filename inside `dir` — separators and `..` are refused so a
/// crafted manifest can never reach outside its own folder (the
/// engine-steps::edit law, held here too).
fn frame_ok(dir: &Path, file: &str) -> Result<PathBuf, String> {
    if file.is_empty() || file.contains('/') || file.contains('\\') || file.contains("..") {
        return Err(format!("invalid frame name \"{file}\""));
    }
    let p = dir.join(file);
    if !p.is_file() {
        return Err(format!("frame not found: \"{file}\""));
    }
    Ok(p)
}

/// A file as a data URI, or an honest error.
fn data_uri(path: &Path, mime: &str) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    Ok(format!("data:{mime};base64,{}", base64(&bytes)))
}

/// JSON serialized for SAFE inline `<script>` embedding: `</` is escaped
/// so no caption can close the script tag.
fn js_json(v: &serde_json::Value) -> String {
    serde_json::to_string(v)
        .unwrap_or_else(|_| "null".into())
        .replace("</", "<\\/")
}

/// Minimal HTML text escaping for the title slots.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// ms → "m:ss" (the chapter labels' clock).
fn mmss(ms: u64) -> String {
    let s = ms / 1000;
    format!("{}:{:02}", s / 60, s % 60)
}

/// Write ONE self-contained `guide.html` beside the manifest's assets in
/// `dir`. Stills always embed; the replay video embeds when
/// `opts.embed_video` (else it references the sibling `session.mp4`, with
/// an honest note when the file is missing at view time). No external
/// requests, no JS deps — hand-rolled minimal script.
pub fn write_html(manifest: &Manifest, dir: &Path, opts: ExportOpts) -> Result<PathBuf, String> {
    if manifest.steps.is_empty() {
        return Err("a guide needs at least one step".into());
    }
    let mut steps_js = Vec::with_capacity(manifest.steps.len());
    for s in &manifest.steps {
        let img = data_uri(&frame_ok(dir, &s.file)?, "image/webp")?;
        let done = match &s.result_file {
            Some(rf) => Some(data_uri(&frame_ok(dir, rf)?, "image/webp")?),
            None => None,
        };
        steps_js.push(serde_json::json!({
            "img": img,
            "done": done,
            "cap": s.caption.clone().unwrap_or_default(),
            "click": s.click,
        }));
    }

    // The replay block: embedded data URI by default (ONE file), sidecar
    // reference otherwise. Chapters ride either way.
    let (video_src, chapters) = match &manifest.session {
        Some(sess) => {
            let src = match &sess.video {
                Some(name) => {
                    let p = frame_ok(dir, name)?;
                    if opts.embed_video {
                        Some(data_uri(&p, "video/mp4")?)
                    } else {
                        Some(name.clone())
                    }
                }
                None => None,
            };
            (src, sess.events.clone())
        }
        None => (None, Vec::new()),
    };

    let title = html_escape(&manifest.name);
    let steps_json = js_json(&serde_json::Value::Array(steps_js));
    let chapter_html: String = chapters
        .iter()
        .map(|t| {
            format!(
                "<button class=\"chap\" data-ms=\"{}\">{} · {}</button>",
                t.ms,
                mmss(t.ms),
                html_escape(&t.label)
            )
        })
        .collect();
    let replay_html = match (&video_src, manifest.session.is_some()) {
        (Some(src), _) => format!(
            "<section id=\"replay\"><h2>Watch it happen</h2>\
             <video id=\"vid\" controls playsinline preload=\"metadata\" src=\"{src}\"></video>\
             <div id=\"chapters\">{chapter_html}</div></section>"
        ),
        (None, true) => format!(
            "<section id=\"replay\"><h2>Watch it happen</h2>\
             <p class=\"note\">This guide recorded a session, but its video file \
             (session.mp4) is not beside this page.</p>\
             <div id=\"chapters\">{chapter_html}</div></section>"
        ),
        (None, false) => String::new(),
    };

    let html = format!(
        r##"<!doctype html>
<html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>{title}</title>
<style>
:root{{--bg:#101319;--panel:#1a1f29;--ink:#e8ecf4;--muted:#8b93a7;--tone:#e6007a;--edge:#2a3040}}
*{{box-sizing:border-box;margin:0;padding:0}}
body{{background:var(--bg);color:var(--ink);font:16px/1.5 system-ui,sans-serif;padding:16px;max-width:860px;margin:0 auto}}
header{{display:flex;align-items:baseline;justify-content:space-between;gap:12px;margin-bottom:12px}}
h1{{font-size:1.25rem}} h2{{font-size:1rem;margin:20px 0 8px;color:var(--muted)}}
.count{{color:var(--muted);font-variant-numeric:tabular-nums}}
#stage{{position:relative;background:var(--panel);border:2px solid var(--edge);border-radius:10px;overflow:hidden}}
#shot{{display:block;width:100%;height:auto}}
#dot{{position:absolute;width:26px;height:26px;border-radius:50%;border:3px solid var(--tone);
box-shadow:0 0 0 3px #0008;transform:translate(-50%,-50%);pointer-events:none;display:none}}
#caption{{min-height:1.5em;margin:10px 2px;color:var(--ink)}}
nav{{display:flex;gap:10px}}
button{{flex:1;min-height:48px;background:var(--panel);color:var(--ink);border:2px solid var(--edge);
border-radius:10px;font-size:1rem;font-weight:600;cursor:pointer;box-shadow:3px 3px 0 #000}}
button:active{{transform:translate(2px,2px);box-shadow:1px 1px 0 #000}}
#next{{border-color:var(--tone)}}
#toggle{{display:none;flex:0 0 auto;padding:0 14px}}
video{{width:100%;border:2px solid var(--edge);border-radius:10px;background:#000}}
#chapters{{display:flex;flex-wrap:wrap;gap:8px;margin-top:8px}}
.chap{{flex:0 1 auto;min-height:40px;padding:4px 10px;font-weight:400;font-size:.85rem}}
.note{{color:var(--muted)}}
</style></head><body>
<header><h1>{title}</h1><p class="count"><span id="n">1</span> / {count}</p></header>
<div id="stage"><img id="shot" alt="step"><div id="dot"></div></div>
<p id="caption"></p>
<nav><button id="prev">&#8249; Back</button><button id="toggle">After</button><button id="next">Next &#8250;</button></nav>
{replay_html}
<script>
"use strict";
const STEPS={steps_json};
let i=0,after=false;
const shot=document.getElementById("shot"),dot=document.getElementById("dot"),
cap=document.getElementById("caption"),n=document.getElementById("n"),
tog=document.getElementById("toggle");
function show(){{
  const s=STEPS[i];
  shot.src=(after&&s.done)?s.done:s.img;
  n.textContent=i+1;
  cap.textContent=s.cap;
  tog.style.display=s.done?"block":"none";
  tog.textContent=after?"Before":"After";
  if(s.click&&!after){{dot.style.display="block";
    dot.style.left=(s.click[0]*100)+"%";dot.style.top=(s.click[1]*100)+"%";}}
  else dot.style.display="none";
}}
function go(d){{const j=i+d;if(j<0||j>=STEPS.length)return;i=j;after=false;show();}}
document.getElementById("prev").addEventListener("click",()=>go(-1));
document.getElementById("next").addEventListener("click",()=>go(1));
tog.addEventListener("click",()=>{{after=!after;show();}});
document.addEventListener("keydown",e=>{{
  if(e.key==="ArrowLeft")go(-1);
  if(e.key==="ArrowRight")go(1);
  if(e.key===" "){{e.preventDefault();go(1);}}
}});
let tx=null;
document.getElementById("stage").addEventListener("touchstart",e=>{{tx=e.changedTouches[0].clientX;}},{{passive:true}});
document.getElementById("stage").addEventListener("touchend",e=>{{
  if(tx===null)return;
  const dx=e.changedTouches[0].clientX-tx;tx=null;
  if(dx<-40)go(1);if(dx>40)go(-1);
}},{{passive:true}});
const vid=document.getElementById("vid");
for(const b of document.querySelectorAll(".chap")){{
  b.addEventListener("click",()=>{{
    if(!vid)return;
    vid.currentTime=Number(b.dataset.ms)/1000;
    vid.play();
  }});
}}
show();
</script></body></html>
"##,
        count = manifest.steps.len(),
    );
    let out = dir.join("guide.html");
    std::fs::write(&out, html).map_err(|e| format!("write {}: {e}", out.display()))?;
    Ok(out)
}

/// Copy one frame from the source guide into the export folder under a
/// new (renumbered) name. Both names are jailed bare filenames.
fn copy_frame(
    src_dir: &Path,
    src_file: &str,
    out_dir: &Path,
    new_name: &str,
) -> Result<(), String> {
    let from = frame_ok(src_dir, src_file)?;
    std::fs::copy(&from, out_dir.join(new_name))
        .map(|_| ())
        .map_err(|e| format!("copy {src_file}: {e}"))
}

/// The full export: source guide (+ optional session folder) → a complete
/// standalone guide folder at `out_dir`. Sources are never mutated —
/// stills are COPIED and renumbered (`step-001.webp` first, so the index
/// thumbnail law holds on the export too), the session video is
/// re-encoded NEW and light, and everything the html references ships in
/// the folder.
pub fn export_guide(
    source: &Path,
    session: Option<&Path>,
    out_dir: &Path,
    opts: ExportOpts,
) -> Result<Export, String> {
    let mut m = load_guide(source)?;
    if m.steps.is_empty() {
        return Err("this guide has no steps".into());
    }
    if source == out_dir {
        return Err("export target must be a NEW folder, not the source guide".into());
    }
    std::fs::create_dir_all(out_dir).map_err(|e| format!("export dir: {e}"))?;
    m.name = out_dir
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or(m.name);

    let mut stills = Vec::with_capacity(m.steps.len());
    for (i, s) in m.steps.iter_mut().enumerate() {
        let order = (i + 1) as u32;
        let file = format!("step-{order:03}.webp");
        copy_frame(source, &s.file, out_dir, &file)?;
        stills.push(out_dir.join(&file));
        s.file = file;
        if let Some(rf) = s.result_file.take() {
            let done = format!("step-{order:03}-done.webp");
            copy_frame(source, &rf, out_dir, &done)?;
            s.result_file = Some(done);
        }
        s.order = order;
    }
    m.step_count = m.steps.len();

    if let Some(sdir) = session {
        if !sdir.is_dir() {
            return Err(format!("not a session folder: {}", sdir.display()));
        }
        let mut sref = load_session(sdir);
        if sref.video.is_some() {
            // The "incredibly light" law: the bundle carries a fresh
            // phone-friendly re-encode, never the original recording.
            dpl_engine_media::render_replay(
                &sdir.join("session.mp4"),
                &out_dir.join("session.mp4"),
            )?;
            sref.video = Some("session.mp4".into());
        }
        m.session = Some(sref);
    }
    if opts.make_gif {
        if let Some(v) = m.session.as_ref().and_then(|s| s.video.as_deref()) {
            // The social clip rides the LIGHT re-encode, never the original.
            dpl_engine_media::render_gif(&out_dir.join(v), &out_dir.join("replay.gif"))?;
            m.replay_gif = Some("replay.gif".into());
        }
    }

    let mut video = None;
    if opts.make_guide_video {
        let session_file = m
            .session
            .as_ref()
            .and_then(|s| s.video.as_deref())
            .map(|v| out_dir.join(v));
        let out = out_dir.join("guide-video.mp4");
        dpl_engine_media::render_guide_video(
            &stills,
            opts.secs_per_still,
            session_file.as_deref(),
            1280,
            720,
            30,
            &out,
        )?;
        m.guide_video = Some("guide-video.mp4".into());
        video = Some(out);
    }

    let html = write_html(&m, out_dir, opts)?;
    std::fs::write(
        out_dir.join("guide.json"),
        serde_json::to_vec_pretty(&m).map_err(|e| format!("manifest: {e}"))?,
    )
    .map_err(|e| format!("guide.json: {e}"))?;
    Ok(Export {
        dir: out_dir.to_path_buf(),
        html,
        video,
        steps: m.steps.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dpl-guide-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A recorder-shaped guide folder: 3 steps (first with a done frame +
    /// click + recorder extras), tiny fake WebP bytes.
    fn fixture_guide(root: &Path) -> PathBuf {
        let dir = root.join("my-guide_v1");
        std::fs::create_dir_all(&dir).unwrap();
        for f in [
            "step-001.webp",
            "step-001-done.webp",
            "step-002.webp",
            "step-005.webp",
        ] {
            std::fs::write(dir.join(f), format!("webp:{f}")).unwrap();
        }
        let manifest = serde_json::json!({
            "kind": "guide", "name": "my-guide_v1", "stepCount": 3, "totalBytes": 30,
            "steps": [
                { "order": 1, "file": "step-001.webp", "resultFile": "step-001-done.webp",
                  "action": "click", "click": [0.25, 0.75], "app": "Test", "title": "Test - App",
                  "width": 100, "height": 80, "bytes": 10 },
                { "order": 2, "file": "step-002.webp", "action": "key",
                  "app": "Test", "title": "Test - App", "width": 100, "height": 80, "bytes": 10 },
                { "order": 3, "file": "step-005.webp", "action": "marked",
                  "app": "Test", "title": "Test - App", "width": 100, "height": 80, "bytes": 10,
                  "caption": "the </script> payoff" },
            ],
        });
        std::fs::write(
            dir.join("guide.json"),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        dir
    }

    fn fixture_session(root: &Path, with_video: bool) -> PathBuf {
        let dir = root.join("session-x");
        std::fs::create_dir_all(&dir).unwrap();
        if with_video {
            std::fs::write(dir.join("session.mp4"), b"mp4-bytes").unwrap();
        }
        std::fs::write(
            dir.join("events.jsonl"),
            concat!(
                "{\"ms\":1000,\"type\":\"route-changed\",\"hub\":\"present\",\"sub\":\"shows\"}\n",
                "{\"ms\":2500,\"type\":\"full-bleed\",\"on\":true}\n",
                "not json\n",
                "{\"ms\":4000,\"type\":\"viziwiz-summon\"}\n",
            ),
        )
        .unwrap();
        dir
    }

    #[test]
    fn base64_matches_known_vectors() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn the_recorder_guide_json_loads_and_round_trips() {
        let root = scratch("load");
        let dir = fixture_guide(&root);
        let m = load_guide(&dir).unwrap();
        assert_eq!(m.kind, "guide");
        assert_eq!(m.step_count, 3);
        assert_eq!(m.steps[0].click, Some([0.25, 0.75]));
        assert_eq!(
            m.steps[0].result_file.as_deref(),
            Some("step-001-done.webp")
        );
        assert_eq!(m.steps[2].caption.as_deref(), Some("the </script> payoff"));
        assert_eq!(m.steps[0].app, "Test", "recorder facts carried");
        // Serialize back: still camelCase, recorder-compatible keys.
        let v: serde_json::Value = serde_json::to_value(&m).unwrap();
        assert_eq!(v["stepCount"], 3);
        assert_eq!(v["steps"][0]["resultFile"], "step-001-done.webp");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn events_map_to_chapter_labels_and_bad_lines_skip() {
        let root = scratch("events");
        let dir = fixture_session(&root, false);
        let s = load_session(&dir);
        assert!(s.video.is_none(), "partial session: no video");
        let labels: Vec<&str> = s.events.iter().map(|t| t.label.as_str()).collect();
        assert_eq!(labels, ["present/shows", "full-bleed on", "viziwiz-summon"]);
        assert_eq!(s.events[0].ms, 1000);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn export_copies_renumbers_and_writes_a_self_contained_html() {
        let root = scratch("export");
        let dir = fixture_guide(&root);
        let out = root.join("my-guide-share");
        let opts = ExportOpts {
            make_guide_video: false, // the ffmpeg leg is the real drill's
            ..Default::default()
        };
        let e = export_guide(&dir, None, &out, opts).unwrap();
        assert_eq!(e.steps, 3);
        // Renumbered: the source's step-005 became step-003.
        assert!(out.join("step-003.webp").is_file());
        assert!(out.join("step-001-done.webp").is_file());
        assert!(!out.join("step-005.webp").exists());
        // The export manifest reflects the new names + the folder name.
        let m = load_guide(&out).unwrap();
        assert_eq!(m.name, "my-guide-share");
        assert_eq!(m.steps[2].file, "step-003.webp");
        assert_eq!(m.steps[2].order, 3);
        // Sources untouched.
        assert!(dir.join("step-005.webp").is_file());
        // The html: one file, all stills embedded, zero external refs.
        let html = std::fs::read_to_string(&e.html).unwrap();
        assert_eq!(
            html.matches("data:image/webp;base64,").count(),
            4,
            "3 do + 1 done"
        );
        assert!(!html.contains("http"), "no external references, ever");
        assert!(
            !html.contains("</script> payoff"),
            "captions can't close the script"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn embed_and_sidecar_modes_treat_the_replay_honestly() {
        let root = scratch("video");
        let dir = fixture_guide(&root);
        // A pre-encoded replay dropped in place stands in for ffmpeg here.
        let out = root.join("share");
        std::fs::create_dir_all(&out).unwrap();
        for s in [
            "step-001.webp",
            "step-001-done.webp",
            "step-002.webp",
            "step-003.webp",
        ] {
            std::fs::write(out.join(s), b"w").unwrap();
        }
        std::fs::write(out.join("session.mp4"), b"light-mp4").unwrap();
        let mut m = load_guide(&dir).unwrap();
        for (i, s) in m.steps.iter_mut().enumerate() {
            s.file = format!("step-{:03}.webp", i + 1);
            if i > 0 {
                s.result_file = None;
            }
        }
        m.session = Some(SessionRef {
            video: Some("session.mp4".into()),
            events: vec![Tick {
                ms: 1500,
                label: "present/shows".into(),
            }],
        });
        let embedded = write_html(&m, &out, ExportOpts::default()).unwrap();
        let html = std::fs::read_to_string(&embedded).unwrap();
        assert!(
            html.contains("data:video/mp4;base64,"),
            "embedded by default"
        );
        assert!(html.contains("data-ms=\"1500\""), "chapter tick rides");
        assert!(html.contains("0:01"), "chapter clock label");
        let sidecar = write_html(
            &m,
            &out,
            ExportOpts {
                embed_video: false,
                ..Default::default()
            },
        )
        .unwrap();
        let html = std::fs::read_to_string(&sidecar).unwrap();
        assert!(html.contains("src=\"session.mp4\""), "sidecar reference");
        assert!(!html.contains("data:video/mp4"), "no embed in sidecar mode");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn refusals_are_honest_and_jailed() {
        let root = scratch("refuse");
        let dir = fixture_guide(&root);
        // A manifest referencing outside the folder is refused by name.
        let mut m = load_guide(&dir).unwrap();
        m.steps[0].file = "../escape.webp".into();
        assert!(write_html(&m, &dir, ExportOpts::default()).is_err());
        // A missing frame is refused.
        let mut m = load_guide(&dir).unwrap();
        m.steps[0].file = "step-099.webp".into();
        assert!(write_html(&m, &dir, ExportOpts::default()).is_err());
        // Exporting onto the source is refused.
        assert!(export_guide(&dir, None, &dir, ExportOpts::default()).is_err());
        // A non-guide folder is refused.
        assert!(load_guide(&root.join("nope")).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }
}
