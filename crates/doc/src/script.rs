//! MDX-lite SCRIPTS (D5) — the teleprompter's data, byte-compatible
//! with the old repo's format: a YAML frontmatter head between `---`
//! fences (slideId · title · beats[{id, cue, note?}]) over a
//! markdown-ish BODY that renders as text and never executes.
//!
//! Home in the new tree: `<slide-folder>/script.mdx`, colocated like
//! ink.json and notes.md (the folder-contract law). Beat ids follow the
//! step semantics — `["setup", ...slide reveals]`, so
//! `beats.len() == slide.step_count()`; parity is a DOCTOR finding,
//! never a crash.
//!
//! No yaml dependency: the frontmatter grammar is OURS and narrow, so a
//! STRICT subset parser reads exactly what we author and refuses —
//! naming the line — anything outside it. The real old-repo corpus pins
//! the compatibility in tests, verbatim.

use crate::{Issue, Slide};
use std::path::Path;

pub const SCRIPT_FILE: &str = "script.mdx";

/// One spoken beat: `id` follows the step semantics, `cue` is the line
/// to LAND verbatim, `note` is delivery guidance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptBeat {
    pub id: String,
    pub cue: String,
    pub note: Option<String>,
}

/// A parsed script: the frontmatter fields + the inert body, verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Script {
    pub slide_id: String,
    pub title: String,
    pub beats: Vec<ScriptBeat>,
    pub body: String,
}

/// Unwrap an optionally double-quoted scalar; inside quotes `\"` and
/// `\\` unescape (the YAML double-quote subset our corpus uses).
fn scalar(raw: &str) -> String {
    let t = raw.trim();
    if t.len() >= 2 && t.starts_with('"') && t.ends_with('"') {
        let inner = &t[1..t.len() - 1];
        let mut out = String::with_capacity(inner.len());
        let mut chars = inner.chars();
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some(other) => {
                        out.push('\\');
                        out.push(other);
                    }
                    None => out.push('\\'),
                }
            } else {
                out.push(c);
            }
        }
        out
    } else {
        t.to_string()
    }
}

/// Parse a script file's full text. Strict: the file must OPEN with a
/// `---` fence; the frontmatter must hold exactly the authored grammar;
/// anything else errs with its line number. The body is everything
/// after the closing fence, verbatim.
pub fn parse_script(raw: &str) -> Result<Script, String> {
    let raw = raw.strip_prefix('\u{feff}').unwrap_or(raw);
    let mut lines = raw.lines().enumerate();
    match lines.next() {
        Some((_, l)) if l.trim_end() == "---" => {}
        _ => return Err("line 1: a script opens with a `---` fence".into()),
    }

    let mut slide_id: Option<String> = None;
    let mut title: Option<String> = None;
    let mut beats: Vec<ScriptBeat> = Vec::new();
    let mut in_beats = false;
    let mut fm_end: Option<usize> = None;

    for (n, line) in lines.by_ref() {
        let human = n + 1;
        if line.trim_end() == "---" {
            fm_end = Some(n);
            break;
        }
        if line.trim().is_empty() {
            continue;
        }
        let indented = line.starts_with(' ');
        let t = line.trim();
        if !indented {
            in_beats = false;
            if let Some(v) = t.strip_prefix("slideId:") {
                slide_id = Some(scalar(v));
            } else if let Some(v) = t.strip_prefix("title:") {
                title = Some(scalar(v));
            } else if t == "beats:" {
                in_beats = true;
            } else {
                return Err(format!(
                    "line {human}: unknown frontmatter key {t:?} — the script grammar is slideId, title, beats"
                ));
            }
            continue;
        }
        if !in_beats {
            return Err(format!("line {human}: indented line outside beats: {t:?}"));
        }
        if let Some(v) = t.strip_prefix("- id:") {
            beats.push(ScriptBeat {
                id: scalar(v),
                cue: String::new(),
                note: None,
            });
        } else if let Some(v) = t.strip_prefix("cue:") {
            match beats.last_mut() {
                Some(b) => b.cue = scalar(v),
                None => return Err(format!("line {human}: cue before any `- id:`")),
            }
        } else if let Some(v) = t.strip_prefix("note:") {
            match beats.last_mut() {
                Some(b) => b.note = Some(scalar(v)),
                None => return Err(format!("line {human}: note before any `- id:`")),
            }
        } else {
            return Err(format!(
                "line {human}: unknown beat line {t:?} — beats carry id, cue, note"
            ));
        }
    }

    let Some(end) = fm_end else {
        return Err("the frontmatter's closing `---` fence is missing".into());
    };
    let slide_id = slide_id.ok_or("frontmatter is missing slideId")?;
    let title = title.ok_or("frontmatter is missing title")?;
    if beats.is_empty() {
        return Err("a script carries at least one beat".into());
    }
    for b in &beats {
        if b.cue.is_empty() {
            return Err(format!("beat {:?} has no cue — cues LAND verbatim", b.id));
        }
    }
    // The body: everything after the closing fence, verbatim — minus at
    // most ONE leading blank line (the conventional gap under the fence).
    let mut body_lines: Vec<&str> = raw.lines().skip(end + 1).collect();
    if body_lines.first().is_some_and(|l| l.trim().is_empty()) {
        body_lines.remove(0);
    }
    let body = body_lines.join("\n");

    Ok(Script {
        slide_id,
        title,
        beats,
        body,
    })
}

/// Load `<slide-folder>/script.mdx`. Absent = Ok(None) — scripts are
/// optional; a present-but-broken script errs with the parser's words.
pub fn load_script(slide_dir: &Path) -> Result<Option<Script>, String> {
    let path = slide_dir.join(SCRIPT_FILE);
    if !path.is_file() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&path).map_err(|e| format!("read {SCRIPT_FILE}: {e}"))?;
    parse_script(&raw).map(Some)
}

/// The beats-parity doctor (old doctrine, restated): expected beat ids
/// are `["setup", ...slide reveals]` — `beats.len() == step_count`.
/// Mismatches are NAMED findings, never a crash.
pub fn parity_issues(slide: &Slide, script: &Script) -> Vec<Issue> {
    let expected: Vec<&str> = std::iter::once("setup")
        .chain(slide.reveals.iter().map(String::as_str))
        .collect();
    let got: Vec<&str> = script.beats.iter().map(|b| b.id.as_str()).collect();
    if expected == got {
        return Vec::new();
    }
    vec![Issue {
        path: "script".into(),
        message: format!(
            "beat ids must follow the step semantics — expected {expected:?}, script has {got:?}"
        ),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The REAL old-repo corpus, verbatim (eos-001 cold-open.mdx) — the
    /// byte-compatibility pin.
    const COLD_OPEN: &str = r#"---
slideId: cold-open
title: Cold open
beats:
  - id: setup
    cue: "You think you know what AI is. You don't. Neither did I."
    note: "Flat. Don't sell it — the screen already said it, you just confirm it. Let *Neither did I* sit a beat. That's the earnest turn; everything after earns off it."
---

Before the tap — let them get comfortable: you've used it, you've got opinions,
you've explained to somebody at a dinner what AI "really is." Let them nod.

[TAP] the line lands. "You don't." Hold it. Then quieter — "Neither did I."
"#;

    #[test]
    fn the_old_corpus_parses_byte_compatible() {
        let s = parse_script(COLD_OPEN).expect("the real corpus must parse");
        assert_eq!(s.slide_id, "cold-open");
        assert_eq!(s.title, "Cold open");
        assert_eq!(s.beats.len(), 1);
        assert_eq!(s.beats[0].id, "setup");
        assert!(s.beats[0].cue.starts_with("You think you know"));
        assert!(s.beats[0]
            .note
            .as_deref()
            .is_some_and(|n| n.contains("*Neither did I*")));
        assert!(s.body.starts_with("Before the tap"));
        assert!(s.body.contains("\"really is.\""), "body stays verbatim");
    }

    #[test]
    fn quoting_unescapes_and_unquoted_scalars_pass_through() {
        assert_eq!(scalar(r#""say \"hi\" \\ done""#), r#"say "hi" \ done"#);
        assert_eq!(scalar("  plain words  "), "plain words");
    }

    #[test]
    fn multiple_beats_with_optional_notes() {
        let raw = "---\nslideId: two\ntitle: Two\nbeats:\n  - id: setup\n    cue: open\n  - id: subtitle\n    cue: land it\n    note: softly\n---\nbody here";
        let s = parse_script(raw).unwrap();
        assert_eq!(s.beats.len(), 2);
        assert_eq!(s.beats[0].note, None);
        assert_eq!(s.beats[1].note.as_deref(), Some("softly"));
        assert_eq!(s.body, "body here");
    }

    #[test]
    fn everything_outside_the_subset_is_refused_by_name() {
        assert!(parse_script("no fence").unwrap_err().contains("line 1"));
        assert!(parse_script("---\nslideId: x\n")
            .unwrap_err()
            .contains("closing"));
        assert!(parse_script("---\nmystery: x\n---\n")
            .unwrap_err()
            .contains("unknown frontmatter key"));
        assert!(
            parse_script("---\nslideId: x\ntitle: t\nbeats:\n  - id: a\n---\n")
                .unwrap_err()
                .contains("no cue")
        );
        assert!(
            parse_script("---\nslideId: x\ntitle: t\nbeats:\n    cue: stray\n---\n")
                .unwrap_err()
                .contains("before any"),
        );
    }

    #[test]
    fn parity_follows_the_step_semantics() {
        let mut slide = Slide::new("title@1");
        slide.reveals = vec!["subtitle".into()];
        let good = parse_script(
            "---\nslideId: s\ntitle: T\nbeats:\n  - id: setup\n    cue: a\n  - id: subtitle\n    cue: b\n---\n",
        )
        .unwrap();
        assert!(parity_issues(&slide, &good).is_empty());
        // A missing reveal beat is named with both id lists.
        slide.reveals.push("kicker".into());
        let issues = parity_issues(&slide, &good);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("kicker"));
        assert!(issues[0].message.contains("subtitle"));
    }

    #[test]
    fn an_absent_script_is_none_not_an_error() {
        let dir = std::env::temp_dir().join("dpl-doc-test-no-script");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(load_script(&dir).unwrap(), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
