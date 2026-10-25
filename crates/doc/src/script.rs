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
//!
//! The card editor writes scripts too (RL, ruled 2026-10-25): its On tap
//! panel keeps the beats following the card's taps ([`follow_reveals`]) and
//! writes the file back ([`save_script`]). A beat whose words are not
//! written yet has an empty cue — it parses, and the doctor names it.

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

/// Point the script in `slide_dir` at the card `id` (LS3b): its `slideId`
/// line is rewritten and every other byte kept. A copied card's script
/// names the card, never the folder it came from. `Ok(false)` when there
/// is no script, or it already names `id`.
pub fn rekey(slide_dir: &Path, id: &str) -> Result<bool, String> {
    let path = slide_dir.join(SCRIPT_FILE);
    if !path.is_file() {
        return Ok(false);
    }
    let raw = std::fs::read_to_string(&path).map_err(|e| format!("read {SCRIPT_FILE}: {e}"))?;
    let script = parse_script(&raw)?;
    if script.slide_id == id {
        return Ok(false);
    }
    let mut out = String::with_capacity(raw.len());
    let mut done = false;
    for line in raw.split_inclusive('\n') {
        if !done && line.starts_with("slideId:") {
            let ending = &line[line.trim_end_matches(['\r', '\n']).len()..];
            out.push_str(&format!("slideId: {id}{ending}"));
            done = true;
        } else {
            out.push_str(line);
        }
    }
    std::fs::write(&path, out).map_err(|e| format!("write {SCRIPT_FILE}: {e}"))?;
    Ok(true)
}

/// The script as its file says it — the inverse of [`parse_script`]: every
/// value double-quoted (`\"` and `\\` escaped), a cue or note kept to one
/// line, the body after one blank line. For a body with no trailing line
/// break, `parse_script(&render_script(s)) == s`.
pub fn render_script(script: &Script) -> String {
    fn quoted(raw: &str) -> String {
        let one_line = raw.replace(['\r', '\n'], " ");
        format!(
            "\"{}\"",
            one_line.replace('\\', "\\\\").replace('"', "\\\"")
        )
    }
    let mut out = String::from("---\n");
    out.push_str(&format!("slideId: {}\n", quoted(&script.slide_id)));
    out.push_str(&format!("title: {}\n", quoted(&script.title)));
    out.push_str("beats:\n");
    for beat in &script.beats {
        out.push_str(&format!("  - id: {}\n", quoted(&beat.id)));
        out.push_str(&format!("    cue: {}\n", quoted(&beat.cue)));
        if let Some(note) = &beat.note {
            out.push_str(&format!("    note: {}\n", quoted(note)));
        }
    }
    out.push_str("---\n");
    if !script.body.is_empty() {
        out.push('\n');
        out.push_str(&script.body);
        out.push('\n');
    }
    out
}

/// Write `<slide-folder>/script.mdx`.
pub fn save_script(slide_dir: &Path, script: &Script) -> Result<(), String> {
    std::fs::write(slide_dir.join(SCRIPT_FILE), render_script(script))
        .map_err(|e| format!("write {SCRIPT_FILE}: {e}"))
}

/// The script with its beats following the card's taps (RL): one beat per
/// `["setup", …reveals]`, each keeping its words by id. A new tap starts
/// with no words; a tap that went away keeps its words in the body under
/// "Removed taps", never silently dropped. No script yet: a fresh one that
/// names the card and is titled `title`.
pub fn follow_reveals(script: Option<&Script>, slide: &Slide, title: &str) -> Script {
    let ids: Vec<&str> = std::iter::once("setup")
        .chain(slide.reveals.iter().map(String::as_str))
        .collect();
    let old: &[ScriptBeat] = script.map_or(&[], |s| s.beats.as_slice());
    let beats = ids
        .iter()
        .map(|id| {
            old.iter()
                .find(|b| b.id == *id)
                .cloned()
                .unwrap_or_else(|| ScriptBeat {
                    id: (*id).to_owned(),
                    cue: String::new(),
                    note: None,
                })
        })
        .collect();
    let mut body = script.map(|s| s.body.clone()).unwrap_or_default();
    let gone: Vec<&ScriptBeat> = old
        .iter()
        .filter(|b| !ids.contains(&b.id.as_str()) && (!b.cue.is_empty() || b.note.is_some()))
        .collect();
    if !gone.is_empty() {
        if !body.is_empty() {
            body.push_str("\n\n");
        }
        body.push_str("Removed taps:");
        for beat in gone {
            body.push_str(&format!("\n- {}: {}", beat.id, beat.cue));
            if let Some(note) = &beat.note {
                body.push_str(&format!(" ({note})"));
            }
        }
    }
    Script {
        slide_id: slide.id.clone(),
        title: script.map_or_else(|| title.to_owned(), |s| s.title.clone()),
        beats,
        body,
    }
}

/// The beats-parity doctor (old doctrine, restated): expected beat ids
/// are `["setup", ...slide reveals]` — `beats.len() == step_count`.
/// Mismatches are NAMED findings, never a crash; so is a beat whose words
/// are not written yet.
pub fn parity_issues(slide: &Slide, script: &Script) -> Vec<Issue> {
    let expected: Vec<&str> = std::iter::once("setup")
        .chain(slide.reveals.iter().map(String::as_str))
        .collect();
    let got: Vec<&str> = script.beats.iter().map(|b| b.id.as_str()).collect();
    let mut issues = Vec::new();
    if expected != got {
        issues.push(Issue {
            path: "script".into(),
            message: format!(
                "beat ids must follow the step semantics — expected {expected:?}, script has {got:?}"
            ),
        });
    }
    for beat in script.beats.iter().filter(|b| b.cue.trim().is_empty()) {
        issues.push(Issue {
            path: format!("script → {}", beat.id),
            message: "no words yet for this tap".into(),
        });
    }
    issues
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
        // A beat with no words yet parses (the editor saves as you type);
        // the doctor names it instead.
        let unwritten = parse_script("---\nslideId: x\ntitle: t\nbeats:\n  - id: setup\n---\n")
            .expect("an empty cue parses");
        assert_eq!(unwritten.beats[0].cue, "");
        let issues = parity_issues(&Slide::new("title@1"), &unwritten);
        assert!(issues.iter().any(|i| i.message.contains("no words yet")));
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

    /// What the editor writes reads back the same, quotes and backslashes
    /// included; a cue is kept to one line.
    #[test]
    fn a_rendered_script_parses_back_to_itself() {
        let script = Script {
            slide_id: "01ABC".into(),
            title: "The \"big\" one".into(),
            beats: vec![
                ScriptBeat {
                    id: "setup".into(),
                    cue: "Say it \\ plainly".into(),
                    note: Some("slow".into()),
                },
                ScriptBeat {
                    id: "subtitle".into(),
                    cue: String::new(),
                    note: None,
                },
            ],
            body: "Two lines\nof notes".into(),
        };
        assert_eq!(parse_script(&render_script(&script)).unwrap(), script);
        let mut multi = script.clone();
        multi.beats[0].cue = "one\ntwo".into();
        assert_eq!(
            parse_script(&render_script(&multi)).unwrap().beats[0].cue,
            "one two"
        );
    }

    /// The beats follow the card's taps by id: words stay with their tap, a
    /// new tap is empty, and a removed tap's words move to the body.
    #[test]
    fn beats_follow_the_cards_taps() {
        let mut slide = Slide::new("title@1");
        slide.reveals = vec!["subtitle".into()];
        let fresh = follow_reveals(None, &slide, "Hello");
        assert_eq!(fresh.slide_id, slide.id);
        assert_eq!(fresh.title, "Hello");
        let ids: Vec<&str> = fresh.beats.iter().map(|b| b.id.as_str()).collect();
        assert_eq!(ids, ["setup", "subtitle"]);
        assert!(fresh.beats.iter().all(|b| b.cue.is_empty()));

        let mut written = fresh.clone();
        written.beats[1].cue = "land it".into();
        written.beats[1].note = Some("softly".into());
        slide.reveals = vec!["eyebrow".into(), "subtitle".into()];
        let moved = follow_reveals(Some(&written), &slide, "ignored");
        let got: Vec<(&str, &str)> = moved
            .beats
            .iter()
            .map(|b| (b.id.as_str(), b.cue.as_str()))
            .collect();
        assert_eq!(
            got,
            [("setup", ""), ("eyebrow", ""), ("subtitle", "land it")]
        );
        assert_eq!(moved.title, "Hello", "an existing script keeps its title");

        slide.reveals = vec!["eyebrow".into()];
        let removed = follow_reveals(Some(&moved), &slide, "ignored");
        assert_eq!(removed.beats.len(), 2);
        assert!(removed.body.contains("Removed taps:"));
        assert!(removed.body.contains("subtitle: land it (softly)"));
        assert!(parity_issues(&slide, &removed)
            .iter()
            .all(|i| !i.message.contains("step semantics")));
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
