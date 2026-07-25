//! The `.mogrt` reader (R2a.2) — a Motion Graphics Template is a plain
//! ZIP holding `definition.json` (capsule metadata + the editable
//! client-control declarations), `project.aegraphic` (Adobe's opaque
//! payload — NEVER parsed here; the `.aep` question stays open by
//! operator ruling), and `thumb.png` (a real preview). Shape verified
//! against real specimens from two unrelated packs (api 1.5 + 1.7).

use serde::Deserialize;
use std::io::Read;
use std::path::Path;

/// What the app can honestly know about a mogrt without After Effects.
#[derive(Debug)]
pub struct Mogrt {
    /// `capsuleName` — the author's real title; file-stem fallback.
    pub name: String,
    /// `capsuleTags` (string entries; anything else is skipped).
    pub tags: Vec<String>,
    /// The embedded `thumb.png` bytes; empty when the zip has none.
    pub thumb_png: Vec<u8>,
}
// NOTE: `definition.json` also declares `clientControls` (editable
// text/font/size — the ratified preset-editing surface). The reader
// grows that field when the controls UI pass consumes it; an unread
// field today would just be dead weight the lint law refuses.

/// `definition.json`, only the fields we read — additive by design.
#[derive(Deserialize, Default)]
struct Definition {
    #[serde(default, rename = "capsuleName")]
    capsule_name: String,
    #[serde(default, rename = "capsuleTags")]
    capsule_tags: Vec<serde_json::Value>,
}

/// Read one mogrt. Absent pieces degrade (stem name, no tags, empty
/// thumb); a file that is not a zip errors honestly.
pub fn read(path: &Path) -> Result<Mogrt, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("open: {e}"))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("not a mogrt zip: {e}"))?;
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();

    let def: Definition = read_entry(&mut archive, "definition.json")
        .and_then(|raw| serde_json::from_slice(&raw).ok())
        .unwrap_or_default();
    let thumb_png = read_entry(&mut archive, "thumb.png").unwrap_or_default();
    let name = if def.capsule_name.trim().is_empty() {
        stem
    } else {
        def.capsule_name
    };
    Ok(Mogrt {
        name,
        tags: def
            .capsule_tags
            .iter()
            .filter_map(|v| v.as_str())
            .map(str::to_string)
            .collect(),
        thumb_png,
    })
}

fn read_entry(archive: &mut zip::ZipArchive<std::fs::File>, name: &str) -> Option<Vec<u8>> {
    let mut entry = archive.by_name(name).ok()?;
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf).ok()?;
    Some(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join("dpl-composer-tests")
            .join(format!("mogrt-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_specimen(path: &Path, def: Option<&str>, thumb: &[u8]) {
        let file = std::fs::File::create(path).unwrap();
        let mut z = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default();
        if let Some(def) = def {
            z.start_file("definition.json", opts).unwrap();
            z.write_all(def.as_bytes()).unwrap();
        }
        z.start_file("project.aegraphic", opts).unwrap();
        z.write_all(b"opaque-egg").unwrap();
        if !thumb.is_empty() {
            z.start_file("thumb.png", opts).unwrap();
            z.write_all(thumb).unwrap();
        }
        z.finish().unwrap();
    }

    #[test]
    fn reads_name_tags_controls_and_thumb() {
        let dir = scratch("full");
        let p = dir.join("Title 01.mogrt");
        write_specimen(
            &p,
            Some(
                r#"{"capsuleName":"Neon Title","capsuleTags":["title","neon",7],
                   "clientControls":[{"type":6},{"type":4}]}"#,
            ),
            b"png-bytes",
        );
        let m = read(&p).unwrap();
        assert_eq!(m.name, "Neon Title");
        assert_eq!(m.tags, vec!["title", "neon"], "non-strings skipped");
        assert_eq!(m.thumb_png, b"png-bytes");
    }

    #[test]
    fn degrades_honestly_without_definition_or_thumb() {
        let dir = scratch("bare");
        let p = dir.join("Mystery.mogrt");
        write_specimen(&p, None, b"");
        let m = read(&p).unwrap();
        assert_eq!(m.name, "Mystery", "file stem fallback");
        assert!(m.tags.is_empty() && m.thumb_png.is_empty());
    }

    #[test]
    fn a_non_zip_errors_honestly() {
        let dir = scratch("junk");
        let p = dir.join("fake.mogrt");
        std::fs::write(&p, b"RIFX not a zip").unwrap();
        let err = read(&p).unwrap_err();
        assert!(err.contains("not a mogrt zip"), "{err}");
    }
}
