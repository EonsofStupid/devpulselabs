//! What can go on a card as a picture — the Picture tab's shelf.
//!
//! The shelf is the artifact INDEX (`dpl_index`), not a file dialog, and that
//! is the design rather than a convenience. Every `Record.rel_path` is already
//! content-root-relative ("inbox/pix.png"), which is exactly what
//! `Block::Media.src` requires and exactly what `dpl_doc`'s content-jail
//! predicate accepts. So **the jail cannot be violated by construction**: this
//! module has no way to produce an absolute path, a drive letter or a `..`,
//! because it never builds a path at all — it passes through one the index
//! already holds.
//!
//! A file dialog would mean either copying files in (a capture concern, and
//! `/content` sources are never mutated) or authoring an escaping ref. Neither
//! belongs in a tab.
//!
//! `show` is taken from the record's `kind`, never sniffed from the extension —
//! the schema authors that discriminator on purpose so the doctor can name a
//! mismatch instead of a renderer guessing.

/// One picture on offer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Picture {
    /// The machine id: content-relative, straight from the index, written
    /// into `Block::Media.src` unchanged. Named `slug` like every other
    /// pickable thing on this surface.
    pub(crate) slug: String,
    /// "image" | "video" — the schema's `as` discriminator, from the record's
    /// own `kind`.
    pub(crate) show: String,
    /// What the operator reads.
    pub(crate) label: String,
}

/// Which sort of picture the shelf is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum PictureKind {
    #[default]
    Image,
    Video,
}

impl PictureKind {
    pub(crate) const ALL: [PictureKind; 2] = [PictureKind::Image, PictureKind::Video];

    /// The index's own `kind` value. Never drawn.
    pub(crate) fn slug(self) -> &'static str {
        match self {
            PictureKind::Image => "image",
            PictureKind::Video => "video",
        }
    }

    /// What the operator reads.
    pub(crate) fn label(self) -> &'static str {
        match self {
            PictureKind::Image => "Pictures",
            PictureKind::Video => "Videos",
        }
    }

    pub(crate) fn tip(self) -> &'static str {
        match self {
            PictureKind::Image => "Still pictures you have already captured or brought in",
            PictureKind::Video => "Clips you have already captured or brought in",
        }
    }
}

/// How many the shelf offers at once. A cap that is never explained reads as
/// "that is all you have", so [`pictures`] reports whether it truncated.
const SHELF: usize = 60;

/// What the shelf found.
pub(crate) struct Shelf {
    pub(crate) items: Vec<Picture>,
    /// True when the index held more than [`SHELF`] and the rest were left
    /// off — said out loud rather than silently dropped.
    pub(crate) more: bool,
    /// Why there is nothing, when there is nothing. `None` when the index
    /// simply has none of this kind.
    pub(crate) unavailable: Option<String>,
}

/// THE census: every picture of one kind the operator already has.
///
/// Reads the index, which is a queryable view over files + sidecars — the
/// files are the truth (`dpl_index`'s own law), so a stale index means a
/// missing tile, never a wrong write.
pub(crate) fn pictures(kind: PictureKind) -> Shelf {
    let index = match dpl_index::Index::open_default() {
        Ok(index) => index,
        // The index is rebuildable and its absence is not the operator's
        // fault, so say what happened rather than showing an empty grid.
        Err(error) => {
            return Shelf {
                items: Vec::new(),
                more: false,
                unavailable: Some(format!(
                    "Your picture list could not be opened, so nothing can be offered yet. \
                     ({error})"
                )),
            }
        }
    };
    // Ask for one more than the shelf so "there are more" is a fact, not a
    // guess from a full page.
    let rows = match index.recent(SHELF + 1, None) {
        Ok(rows) => rows,
        Err(error) => {
            return Shelf {
                items: Vec::new(),
                more: false,
                unavailable: Some(format!("Your picture list could not be read. ({error})")),
            }
        }
    };
    let mut items: Vec<Picture> = rows
        .into_iter()
        .filter(|r| r.kind == kind.slug())
        .map(|r| Picture {
            label: if r.name.trim().is_empty() {
                r.rel_path.clone()
            } else {
                r.name.clone()
            },
            show: r.kind,
            slug: r.rel_path,
        })
        .collect();
    let more = items.len() > SHELF;
    items.truncate(SHELF);
    Shelf {
        items,
        more,
        unavailable: None,
    }
}

/// Plain words for the two fit values the schema allows.
pub(crate) fn fit_label(fit: &str) -> &'static str {
    match fit {
        "contain" => "Show all of it",
        // "cover" and any unknown read as the schema's default.
        _ => "Fill the card",
    }
}

/// The one control that is not a picture: take it off.
pub(crate) const NOTHING: &str = "No picture";

#[cfg(test)]
mod tests {
    use super::*;

    /// The census law: the filter row IS the schema's `as` vocabulary, so the
    /// tab can never offer a kind the document cannot hold.
    #[test]
    fn the_kind_row_is_the_schemas_media_vocabulary() {
        let slugs: Vec<&str> = PictureKind::ALL.iter().map(|k| k.slug()).collect();
        assert_eq!(slugs, ["image", "video"]);
    }

    /// The UI law.
    #[test]
    fn no_kind_speaks_engineering() {
        for kind in PictureKind::ALL {
            let drawn = format!("{} {}", kind.label(), kind.tip()).to_lowercase();
            for word in ["slug", "kind", "json", "src", "rel_path", "enum"] {
                assert!(!drawn.contains(word), "{drawn:?} says {word:?}");
            }
        }
    }

    /// Both fit values have a plain word, and an unknown one degrades to the
    /// schema's default rather than showing a ref.
    #[test]
    fn every_fit_has_a_plain_word() {
        assert_eq!(fit_label("cover"), "Fill the card");
        assert_eq!(fit_label("contain"), "Show all of it");
        assert_eq!(fit_label("nonsense"), "Fill the card");
    }

    /// Whatever the shelf offers must be writable into a document — the jail
    /// proven by construction rather than by hope. This runs against whatever
    /// index the machine has, including none.
    #[test]
    fn every_offered_picture_is_a_legal_media_ref() {
        for kind in PictureKind::ALL {
            let shelf = pictures(kind);
            for item in &shelf.items {
                let slide = dpl_doc::Slide {
                    v: dpl_doc::SCHEMA_V,
                    template: "media".into(),
                    background: None,
                    blocks: vec![dpl_doc::Block::Media {
                        id: "picture".into(),
                        show: item.show.clone(),
                        src: item.slug.clone(),
                        fit: None,
                        poster: None,
                    }],
                    enter: None,
                    reveals: Vec::new(),
                };
                assert!(
                    dpl_doc::validate(&slide).is_empty(),
                    "{} is offered but does not validate: {:?}",
                    item.slug,
                    dpl_doc::validate(&slide)
                );
            }
        }
    }
}
