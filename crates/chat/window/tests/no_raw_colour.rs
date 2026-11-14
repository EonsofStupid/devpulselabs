//! The chat window paints only through ColdLight: every colour it uses is a
//! colour role (`dpl_coldlight::color_roles::chat`), a tone, or a named
//! surface or ink token. This test reads the window's own source and fails
//! the moment a hand-typed colour comes back, or a glyph the bundled fonts
//! cannot draw (an empty box on screen) replaces a drawn icon.

use std::path::Path;

/// Constructors that turn numbers into a colour. Assembled from parts so this
/// file does not trip the scan it performs.
fn raw_colour_needles() -> Vec<String> {
    let colour = ["Color", "32::"].concat();
    let mut needles: Vec<String> = [
        "from_rgb(",
        "from_rgba_unmultiplied(",
        "from_rgba_unmultiplied_const(",
        "from_rgba_premultiplied(",
        "from_gray(",
        "from_hex(",
    ]
    .iter()
    .map(|ctor| format!("{colour}{ctor}"))
    .collect();
    needles.push(["from_black", "_alpha("].concat());
    needles.push(["from_white", "_alpha("].concat());
    needles.push(["Rgba", "::from_"].concat());
    needles
}

/// Maximize/restore boxes and disclosure triangles: none of the bundled fonts
/// carries them, so they paint as tofu. Written as escapes for the same
/// reason as above.
const TOFU: [char; 4] = ['\u{25A1}', '\u{2750}', '\u{25BE}', '\u{25B8}'];

fn sources() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    let mut stack = vec![dir];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("read the window's src") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                let text = std::fs::read_to_string(&path).expect("read a source file");
                files.push((path.display().to_string(), text));
            }
        }
    }
    assert!(!files.is_empty(), "the scan must find the window's sources");
    files
}

#[test]
fn the_chat_window_paints_no_raw_colour() {
    let needles = raw_colour_needles();
    let mut found = Vec::new();
    for (path, text) in sources() {
        for (number, line) in text.lines().enumerate() {
            if let Some(needle) = needles.iter().find(|needle| line.contains(needle.as_str())) {
                found.push(format!("{path}:{} uses {needle}", number + 1));
            }
        }
    }
    assert!(
        found.is_empty(),
        "raw colour in crates/chat/window — bind a colour role in tokens/roles.json \
         (`chat.colors`) instead:\n{}",
        found.join("\n")
    );
}

#[test]
fn the_chat_window_draws_icons_not_missing_glyphs() {
    let mut found = Vec::new();
    for (path, text) in sources() {
        for (number, line) in text.lines().enumerate() {
            if let Some(glyph) = TOFU.iter().find(|glyph| line.contains(**glyph)) {
                found.push(format!(
                    "{path}:{} carries U+{:04X}",
                    number + 1,
                    *glyph as u32
                ));
            }
        }
    }
    assert!(
        found.is_empty(),
        "glyphs the bundled fonts cannot draw — use a registry icon from \
         tokens/icons.json:\n{}",
        found.join("\n")
    );
}
