//! Authoring surface — say what you want, then sweep and tap.
//!
//! Pass A landed the spine: an intent bar and one honest mutation — a draft
//! card is a REAL, schema-valid `slide.json` in the content inbox, and
//! `BuildEvent::CardStarted` publishes AFTER the write (emit-with-mutation).
//!
//! This pass adds the way you actually pick things: a **shelf picker** over
//! **one carousel** of large, sweepable cards — the operator's ask, "easily
//! able to select from premades … so I could do basic creation on the TV via
//! touch". Three cards across whatever the window, so the targets stay
//! thumb-sized on the 75" prop.
//!
//! Every shelf is drawn LIVE from the real registries (`demos()`,
//! `elements()`, `blocks()`, `cl::TRAITS`, `list_seeds()`) — the Census law.
//! Nothing is hand-listed, so a piece shows up the day it registers, and a
//! shelf with nothing on it says why rather than looking broken.
//!
//! Vocabulary: the shelves read Starters · Layouts · Combos · Pieces · Themes ·
//! Interactions. What the code files those under lives in [`Shelf::slug`] and
//! appears nowhere on screen (the UI law, DS3).
//!
//! Law: the surface PUBLISHES `build.*` and never SUBSCRIBES to it
//! (anti-loop) — the coachable-signal catalog forwards the fact to touchpoints
//! so AI/coach overlays react without the surface self-triggering.

use crate::containers::{containers, Container, ContainerKind};
use crate::controls::{control, stacked_control, Weight};
use crate::words::pretty;
use dpl_coldlight::{self as cl, Tone};
use dpl_slides::SlideTemplate;
use egui::RichText;
use egui_pilot_ai::{NodeMetadata, PilotId, PilotKey, UiPilotExt};

/// How many extra cards one sweep adds when the shelf is set to keep going.
const PAGE: usize = 6;

/// Tones a theme-plate cycles through down the shelf, so a sweep reads as
/// motion rather than a row of photocopies.
const TONE_CYCLE: [Tone; 4] = [Tone::Cyan, Tone::Amber, Tone::Magenta, Tone::Coral];

/// One tile in the filmstrip: the card drawn as a miniature, with its name
/// under it.
///
/// It used to be a labelled plate, because a whole slide squeezed into a
/// thumb-sized stage cropped — the ramp has a floor. `DocSlide::thumbnail_ui`
/// removed that, so the tile now shows the real document.
///
/// It keeps the NAME as well as the picture. Three freshly-started cards theme
/// alike as pictures — the caption is what tells them apart, which is why the
/// ratified mock's film card carries both. A card whose document will not
/// load keeps the caption and shows an empty stage rather than pretending.
fn film_tile(
    ui: &mut egui::Ui,
    w: f32,
    h: f32,
    label: &str,
    tone: Tone,
    doc: &mut Option<dpl_slides::document::DocSlide>,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::click());
    let style = cl::compose(
        tone,
        "claymorphic",
        &[],
        if resp.hovered() {
            cl::TraitState::Hover
        } else {
            cl::TraitState::Idle
        },
    );
    let fluid = cl::Fluid::of(rect);
    let clip = rect.intersect(ui.clip_rect());
    let painter = ui.painter().with_clip_rect(clip);
    for sh in &style.shadows {
        painter.rect_filled(
            rect.translate(egui::vec2(sh.offset[0] as f32, sh.offset[1] as f32)),
            egui::CornerRadius::same(style.radius),
            sh.color,
        );
    }
    painter.rect_filled(rect, egui::CornerRadius::same(style.radius), style.fill);

    // The picture on top, the name under it — the mock's film card.
    let caption_h = fluid.fs_body_sm() + cl::space::S2;
    let inset = style.edge.width + 1.0;
    let stage = egui::Rect::from_min_max(
        rect.min + egui::vec2(inset, inset),
        egui::pos2(rect.max.x - inset, rect.max.y - caption_h),
    );
    if let Some(doc) = doc {
        if stage.width() > 1.0 && stage.height() > 1.0 {
            let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(stage));
            inner.set_clip_rect(stage.intersect(clip));
            doc.thumbnail_ui(&mut inner, stage);
        }
    }
    painter.text(
        egui::pos2(rect.center().x, rect.max.y - caption_h * 0.5),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(fluid.fs_body_sm()),
        if resp.hovered() {
            tone.color()
        } else {
            style.ink
        },
    );
    painter.rect_stroke(
        rect,
        egui::CornerRadius::same(style.radius),
        style.edge,
        egui::StrokeKind::Inside,
    );
    resp
}

/// The most recently made card in the library, if there is one. A slide's
/// id sorts by the time it was made, so the newest really is the last.
fn newest_draft() -> Option<std::path::PathBuf> {
    dpl_doc::library_slides(&dpl_engine_capture::content_root())
        .into_iter()
        .next()
}

/// One tab chip. Shared by both rail orientations so a reworded tab, a pilot
/// key or the selected styling can never drift between them.
fn tab_chip(ui: &mut egui::Ui, ed: &mut CardEditor, t: CardTab, tone: Tone) {
    let active = ed.tab == t;
    let chip = cl::tip(
        control(ui, t.label(), active, tone, Weight::Chrome),
        t.tip(),
    );
    ui.pilot_track(
        &chip,
        NodeMetadata::new(
            PilotId::new("workbench.author.tab").child(&PilotKey::new(t.slug())),
            egui_pilot_ai::protocol::Role::Tab,
            t.label(),
        )
        .value(if active { "active" } else { "available" })
        .selected(active),
    );
    if chip.clicked() {
        ed.tab = t;
    }
}

/// The rail as R1 draws it — one tab per line down the left.
fn tab_rail(ui: &mut egui::Ui, ed: &mut CardEditor, tone: Tone) {
    for t in CardTab::ALL {
        tab_chip(ui, ed, t, tone);
        ui.add_space(cl::space::S2);
    }
}

/// The rail when there is no room for a column — wrapped across the top, the
/// same degrade R1 makes on a narrow window.
fn tab_rail_wrapped(ui: &mut egui::Ui, ed: &mut CardEditor, tone: Tone) {
    ui.horizontal_wrapped(|ui| {
        for t in CardTab::ALL {
            tab_chip(ui, ed, t, tone);
            ui.add_space(cl::space::S3);
        }
    });
}

/// The live pane: the card as it looks right now.
///
/// Drawn from the editor's OWN in-memory slide, not a re-read of the file, so
/// a tapped colour shows on the next frame rather than after a save
/// round-trip. `thumbnail_ui` is what makes it possible at pane size — before
/// #147 a whole slide in a 300pt stage cropped.
fn live_pane(ui: &mut egui::Ui, ed: &mut CardEditor) {
    let fluid = cl::Fluid::of(ui.ctx().content_rect());
    ui.horizontal(|ui| {
        cl::tone_dot(ui, Tone::Cyan);
        ui.label(
            RichText::new("as it looks right now")
                .size(fluid.fs_eyebrow())
                .color(cl::ink::MUTED),
        );
    });
    ui.add_space(cl::space::S2);

    let w = ui.available_width();
    let (stage, _) = ui.allocate_exact_size(egui::vec2(w, w * 9.0 / 16.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(stage, cl::RADIUS, cl::surface::WELL);
    {
        let now = ui.input(|i| i.time);
        let t0 = ed.preview.tick(now);
        let inner_rect = stage.shrink(1.0);
        let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(inner_rect));
        inner.set_clip_rect(stage.intersect(ui.clip_rect()));
        dpl_slides::document::render_thumbnail(
            &mut inner,
            inner_rect,
            ed.editor.slide(),
            ed.editor.theme(),
            now,
            t0,
        );
    }
    ui.painter().rect_stroke(
        stage,
        cl::RADIUS,
        egui::Stroke::new(1.0, cl::surface::HAIRLINE),
        egui::StrokeKind::Inside,
    );

    ui.add_space(cl::space::S2);
    ui.label(
        RichText::new("Every change lands here as you make it.")
            .size(fluid.fs_body_sm())
            .color(cl::ink::SUBTLE),
    );
}

/// The Picture tab — an image or a video on the card.
///
/// The shelf is the artifact INDEX, not a file dialog. That is the design:
/// every path the index holds is already content-root-relative, so the media
/// content-jail is satisfied by construction (`pictures.rs`). A dialog would
/// mean copying files in — a capture concern, and sources are never mutated —
/// or authoring a ref that escapes the root.
fn picture_tab(ui: &mut egui::Ui, ed: &mut CardEditor) {
    let fluid = cl::Fluid::of(ui.ctx().content_rect());
    let current = ed.editor.media();
    if ed.shelf.is_none() {
        ed.shelf = Some(crate::pictures::pictures(ed.pictures));
    }
    let mut want: Option<Option<(String, String, String)>> = None;

    ui.add_space(cl::space::S2);
    ui.horizontal_wrapped(|ui| {
        for kind in crate::pictures::PictureKind::ALL {
            let on = ed.pictures == kind;
            if cl::tip(ui.selectable_label(on, kind.label()), kind.tip()).clicked() && !on {
                ed.pictures = kind;
                // The gesture that changes the shelf is the one that re-reads
                // it.
                ed.shelf = None;
            }
        }
        ui.add_space(cl::space::S3);
        if current.is_some()
            && cl::tip(
                ui.selectable_label(false, crate::pictures::NOTHING),
                "Take the picture off this card",
            )
            .clicked()
        {
            want = Some(None);
        }
    });
    ui.add_space(cl::space::S2);

    // Two different emptinesses, and they must not read alike: the list could
    // not be opened, versus you simply have none of this kind yet.
    // Degrade, never crash: a paint path does not panic on its own state.
    let Some(shelf) = ed.shelf.as_ref() else {
        return;
    };
    if let Some(why) = &shelf.unavailable {
        ui.label(RichText::new(why).color(cl::ink::MUTED));
        return;
    }
    if shelf.items.is_empty() {
        ui.label(
            RichText::new(format!(
                "No {} in your content folder yet.",
                ed.pictures.label().to_lowercase()
            ))
            .size(fluid.fs_body_lg())
            .strong()
            .color(cl::ink::STRONG),
        );
        ui.add_space(cl::space::S2);
        ui.label(
            RichText::new(
                "Snip something, or bring a folder in, and it shows up here. Nothing is \
                 copied or changed when you pick it — the card just points at it.",
            )
            .color(cl::ink::MUTED),
        );
        return;
    }

    // How it sits on the card. Only meaningful once there IS one.
    if let Some((src, show, fit)) = &current {
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("How it sits")
                    .size(fluid.fs_body())
                    .color(cl::ink::BASE),
            );
            ui.add_space(cl::space::S2);
            for option in ["cover", "contain"] {
                let on = fit == option;
                if cl::tip(
                    ui.selectable_label(on, crate::pictures::fit_label(option)),
                    if option == "cover" {
                        "Fills the whole card, cropping the edges if it has to"
                    } else {
                        "Shows the whole picture, with space around it"
                    },
                )
                .clicked()
                    && !on
                {
                    want = Some(Some((src.clone(), show.clone(), option.to_string())));
                }
            }
        });
        ui.add_space(cl::space::S3);
    }

    let tile_w = fluid.ctl_thumb_w() * 1.6;
    let tile_h = tile_w * 9.0 / 16.0;
    ui.horizontal_wrapped(|ui| {
        for item in &shelf.items {
            let picked = current
                .as_ref()
                .is_some_and(|(src, _, _)| *src == item.slug);
            let (rect, resp) =
                ui.allocate_exact_size(egui::vec2(tile_w, tile_h), egui::Sense::click());
            let clip = rect.intersect(ui.clip_rect());
            {
                let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(rect));
                inner.set_clip_rect(clip);
                // The same ladder the card uses — locate, paint, or an honest
                // plate. A record whose file has gone shows as missing here
                // exactly as it would there.
                dpl_slides::elements::asset_thumb(&mut inner, rect, &item.slug);
            }
            let painter = ui.painter().with_clip_rect(clip);
            painter.rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(rect.min.x, rect.max.y - cl::space::S5),
                    rect.max,
                ),
                egui::CornerRadius::same(cl::RADIUS),
                cl::scrim(160),
            );
            painter.text(
                egui::pos2(rect.center().x, rect.max.y - cl::space::S5 * 0.5),
                egui::Align2::CENTER_CENTER,
                &item.label,
                egui::FontId::proportional(cl::Fluid::of(rect).fs_body_sm()),
                cl::ink::STRONG,
            );
            painter.rect_stroke(
                rect,
                egui::CornerRadius::same(cl::RADIUS),
                egui::Stroke::new(
                    if picked { 2.0 } else { 1.0 },
                    if picked {
                        super::SPEC.tone.color()
                    } else {
                        cl::surface::HAIRLINE
                    },
                ),
                egui::StrokeKind::Inside,
            );
            resp.widget_info(|| {
                egui::WidgetInfo::selected(egui::WidgetType::Button, true, picked, &item.label)
            });
            let tip = if picked {
                format!("{} — this is what the card has now", item.label)
            } else {
                format!("Put “{}” on the card", item.label)
            };
            if cl::tip(resp, &tip).clicked() && !picked {
                want = Some(Some((
                    item.slug.clone(),
                    item.show.clone(),
                    current
                        .as_ref()
                        .map(|(_, _, f)| f.clone())
                        .unwrap_or_else(|| "cover".to_string()),
                )));
            }
            ui.add_space(cl::space::S2);
        }
    });

    if shelf.more {
        ui.add_space(cl::space::S2);
        ui.label(
            RichText::new("Showing the most recent — there are more than fit here.")
                .size(fluid.fs_body_sm())
                .color(cl::ink::SUBTLE),
        );
    }

    if let Some(next) = want {
        ed.editor.set_media(next);
    }
}

/// The "A quote" tab — someone else's words, kept with where they came from.
///
/// A quote is a CONVENTION over text blocks, ratified by the operator on
/// 2026-07-29 and declared in `dpl_doc::quote_lines()`: three reserved ids,
/// ordinary blocks, nothing special-cased at render. The tab is therefore
/// three fields, and the live pane beside it shows the result as it is typed.
///
/// Editing is debounced through the tab's own buffer rather than writing on
/// every keystroke: `set_quote` snapshots for undo, so a per-character write
/// would fill the undo stack with fifty entries for one sentence.
fn quote_tab(ui: &mut egui::Ui, ed: &mut CardEditor) {
    let fluid = cl::Fluid::of(ui.ctx().content_rect());
    // Adopt the document's own words the first time this card's tab is shown,
    // so the fields reflect the card rather than the last one edited.
    if ed.quote.is_none() {
        ed.quote = Some(ed.editor.quote());
    }
    let Some(lines) = ed.quote.as_mut() else {
        return;
    };

    ui.add_space(cl::space::S2);
    ui.label(
        RichText::new("Someone else's words")
            .size(fluid.fs_body_lg())
            .strong()
            .color(cl::ink::STRONG),
    );
    ui.label(
        RichText::new("Leave a line empty and it comes off the card.")
            .small()
            .color(cl::ink::MUTED),
    );
    ui.add_space(cl::space::S3);

    let mut changed = false;
    for (slot, label, hint) in [
        (0usize, "What was said", "the words themselves"),
        (1, "Who said it", "a name, or who they are"),
        (2, "Where it's from", "a talk, a page, a date"),
    ] {
        ui.label(
            RichText::new(label)
                .size(fluid.fs_body())
                .color(cl::ink::BASE),
        );
        let edit = if slot == 0 {
            ui.add(
                egui::TextEdit::multiline(&mut lines[slot])
                    .desired_rows(3)
                    .hint_text(hint)
                    .desired_width(f32::INFINITY),
            )
        } else {
            ui.add(
                egui::TextEdit::singleline(&mut lines[slot])
                    .hint_text(hint)
                    .desired_width(f32::INFINITY),
            )
        };
        changed |= edit.changed();
        cl::tip(edit, hint);
        ui.add_space(cl::space::S3);
    }

    // Written when the field loses focus or a frame after a change settles —
    // one undo entry per edit, not one per character.
    if changed {
        let next = lines.clone();
        ed.editor.set_quote(next);
    }

    // Say what this card is, so adding a quote to a title card is a choice
    // rather than a surprise.
    let other: Vec<String> = ed
        .editor
        .slide()
        .blocks
        .iter()
        .map(|b| b.id().to_string())
        .filter(|id| !dpl_editor::QUOTE_SLOTS.contains(&id.as_str()))
        .collect();
    if !other.is_empty() {
        ui.add_space(cl::space::S2);
        ui.label(
            RichText::new(format!(
                "This card already has {} of its own, and they stay. A quote is added \
                 alongside them.",
                if other.len() == 1 {
                    "a line".to_string()
                } else {
                    format!("{} lines", other.len())
                }
            ))
            .size(fluid.fs_body_sm())
            .color(cl::ink::SUBTLE),
        );
    }
}

/// The Movement tab — how the words arrive.
///
/// THREE choices, and that is the whole truth of it. The document permits two
/// entrances (`dpl_doc::enters()`) and the renderer implements one; the third
/// choice is to follow the show, which is a real answer and not an absence.
///
/// The seventeen other motion effects in the tokens belong to TEMPLATES, for
/// their own elements — the document path consumes `ENTER_RISE` alone.
/// Offering them here would offer refs the document cannot hold.
///
/// The shortfall is stated under the choices rather than hidden. An operator
/// should leave this tab knowing what exists, not wondering whether the UI is
/// broken.
fn movement_tab(ui: &mut egui::Ui, ed: &mut CardEditor) {
    let editor = &mut ed.editor;
    let fluid = cl::Fluid::of(ui.ctx().content_rect());
    let current = editor.slide().enter.clone();
    let mut want: Option<Option<String>> = None;

    ui.add_space(cl::space::S2);
    ui.label(
        RichText::new("How the words arrive")
            .size(fluid.fs_body_lg())
            .strong()
            .color(cl::ink::STRONG),
    );
    ui.add_space(cl::space::S2);

    // "Follow the show" first: inheriting is the default and the commonest
    // right answer, so it should not read as the leftover option.
    let inherited = editor.look_enter();
    let follow = cl::tip(
        ui.selectable_label(
            current.is_none(),
            format!("Follow the show — {}", movement_words(inherited.as_deref())),
        ),
        "Use whatever this show does. Change the show and this card follows.",
    );
    if follow.clicked() {
        want = Some(None);
    }
    for enter in dpl_doc::enters() {
        let on = current.as_deref() == Some(*enter);
        let hit = cl::tip(
            ui.selectable_label(on, movement_label(enter)),
            movement_tip(enter),
        );
        if hit.clicked() {
            want = Some(Some((*enter).to_string()));
        }
    }

    ui.add_space(cl::space::S4);
    ui.label(
        RichText::new("Arriving is all that is built")
            .size(fluid.fs_body())
            .strong()
            .color(cl::ink::BASE),
    );
    ui.label(
        RichText::new(
            "Getting noticed and going out are not. Each one needs a movement authored, \
             the card's file taught to name it, and the player taught to play it — so \
             they arrive together or not at all.",
        )
        .color(cl::ink::MUTED),
    );

    if let Some(next) = want {
        editor.set_enter(next);
    }
}

/// The operator's word for an entrance ref. `slug` never reaches the screen.
fn movement_label(slug: &str) -> &'static str {
    match slug {
        "rise" => "Rise into place",
        "none" => "Stay still",
        // A new entrance allowed by the schema before anyone wrote its word:
        // say so rather than printing the ref.
        _ => "A new way in",
    }
}

fn movement_tip(slug: &str) -> &'static str {
    match slug {
        "rise" => "The words lift into place, one after another",
        "none" => "The words are simply there — nothing moves",
        _ => "A movement this build does not have a word for yet",
    }
}

/// What "follow the show" currently amounts to, so the choice is legible
/// rather than an invisible default.
fn movement_words(inherited: Option<&str>) -> String {
    match inherited {
        Some(slug) => movement_label(slug).to_lowercase(),
        None => "nothing set".to_string(),
    }
}

/// The Background tab — what sits behind the words.
///
/// Every tile is a LIVE render of the backdrop it offers, painted through the
/// same path the card uses, so what you tap is what you get. A picker showing
/// an approximation of the result is the friction this whole surface exists
/// to remove.
///
/// The rows arrive in the order a person decides: pick a colour, then
/// optionally blend a second, then optionally set something moving. Rows two
/// and three stay hidden until row one is answered, so the first decision is
/// never buried under two that depend on it.
fn background_tab(ui: &mut egui::Ui, ed: &mut CardEditor) {
    let editor = &mut ed.editor;
    let fluid = cl::Fluid::of(ui.ctx().content_rect());
    let colours = crate::backgrounds::colours();
    let current = editor.slide().background.clone();

    // What is picked right now, read off the document rather than remembered
    // beside it — the file is the truth.
    let (first, second, ambience) = match &current {
        Some(dpl_doc::Background::Mood { mood }) => (Some(mood.clone()), None, None),
        Some(dpl_doc::Background::MoodBlend {
            moods, ambience, ..
        }) => (
            moods.first().cloned(),
            moods.get(1).cloned(),
            ambience.clone(),
        ),
        _ => (None, None, None),
    };

    ui.add_space(cl::space::S2);
    ui.label(
        RichText::new(format!("The colour — {} to choose from", colours.len()))
            .size(fluid.fs_body_lg())
            .strong()
            .color(cl::ink::STRONG),
    );
    ui.add_space(cl::space::S2);

    let mut want: Option<Option<dpl_doc::Background>> = None;

    // Row one: the colour. "Nothing" sits first because a bare card is a real
    // answer, not a failure to choose.
    let tile_w = fluid.ctl_thumb_w() * 1.6;
    let tile_h = tile_w * 9.0 / 16.0;
    ui.horizontal_wrapped(|ui| {
        if swatch(
            ui,
            tile_w,
            tile_h,
            None,
            first.is_none(),
            crate::backgrounds::NOTHING,
        )
        .clicked()
        {
            want = Some(Some(dpl_doc::Background::None));
        }
        for colour in &colours {
            let picked = first.as_deref() == Some(colour.slug.as_str());
            if swatch(
                ui,
                tile_w,
                tile_h,
                Some(&colour.slug),
                picked,
                &colour.label,
            )
            .clicked()
            {
                want = Some(Some(dpl_doc::Background::Mood {
                    mood: colour.slug.clone(),
                }));
            }
        }
    });

    // Rows two and three only exist once there is something to blend WITH.
    if let Some(base) = first.clone() {
        ui.add_space(cl::space::S4);
        ui.label(
            RichText::new("Blend a second into it")
                .size(fluid.fs_body_lg())
                .strong()
                .color(cl::ink::STRONG),
        );
        ui.label(
            RichText::new("Tap the one that is already on to go back to a single colour.")
                .small()
                .color(cl::ink::MUTED),
        );
        ui.add_space(cl::space::S2);
        let mix = crate::backgrounds::mixes()
            .first()
            .map(|m| m.slug.clone())
            .unwrap_or_default();
        ui.horizontal_wrapped(|ui| {
            for colour in &colours {
                if colour.slug == base {
                    continue;
                }
                let picked = second.as_deref() == Some(colour.slug.as_str());
                if swatch(
                    ui,
                    tile_w,
                    tile_h,
                    Some(&colour.slug),
                    picked,
                    &colour.label,
                )
                .clicked()
                {
                    want = Some(Some(if picked {
                        dpl_doc::Background::Mood { mood: base.clone() }
                    } else {
                        dpl_doc::Background::MoodBlend {
                            moods: vec![base.clone(), colour.slug.clone()],
                            motion: mix.clone(),
                            ambience: ambience.clone(),
                        }
                    }));
                }
            }
        });

        ui.add_space(cl::space::S4);
        ui.label(
            RichText::new("Something moving over it")
                .size(fluid.fs_body_lg())
                .strong()
                .color(cl::ink::STRONG),
        );
        ui.add_space(cl::space::S2);
        ui.horizontal_wrapped(|ui| {
            let moving = crate::backgrounds::ambiences();
            let still = cl::tip(
                ui.selectable_label(ambience.is_none(), "Still"),
                "Nothing drifting over the colour",
            );
            if still.clicked() {
                want = Some(Some(background_with(&base, &second, &mix, None)));
            }
            for choice in &moving {
                let on = ambience.as_deref() == Some(choice.slug.as_str());
                let hit = cl::tip(
                    ui.selectable_label(on, &choice.label),
                    "Tap to put this over the colour",
                );
                if hit.clicked() {
                    let next = (!on).then(|| choice.slug.clone());
                    want = Some(Some(background_with(&base, &second, &mix, next)));
                }
            }
        });
    }

    if let Some(next) = want {
        editor.set_background(next);
    }
}

/// Rebuild a background around a possibly-changed ambience, keeping whatever
/// colours are already picked. An ambience rides on a blend, so a single
/// colour is promoted to a blend with itself rather than losing the movement.
fn background_with(
    base: &str,
    second: &Option<String>,
    mix: &str,
    ambience: Option<String>,
) -> dpl_doc::Background {
    match (second, &ambience) {
        (None, None) => dpl_doc::Background::Mood {
            mood: base.to_string(),
        },
        (Some(other), _) => dpl_doc::Background::MoodBlend {
            moods: vec![base.to_string(), other.clone()],
            motion: mix.to_string(),
            ambience,
        },
        (None, Some(_)) => dpl_doc::Background::MoodBlend {
            moods: vec![base.to_string(), base.to_string()],
            motion: mix.to_string(),
            ambience,
        },
    }
}

/// One background option, painted for real.
///
/// `mood` of `None` is the bare well — the honest picture of "nothing behind
/// the words", not an empty box.
fn swatch(
    ui: &mut egui::Ui,
    w: f32,
    h: f32,
    mood: Option<&str>,
    picked: bool,
    label: &str,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::click());
    let radius = egui::CornerRadius::same(cl::RADIUS);
    let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    match mood.and_then(|name| cl::MOODS.iter().find(|m| m.name == name)) {
        Some(m) => cl::mood_gradient(&painter, rect, m),
        None => {
            painter.rect_filled(rect, radius, cl::surface::WELL);
        }
    }
    // The name rides ON the colour, over a scrim, because a caption beneath
    // would double the height of a row that is already ten wide.
    painter.rect_filled(
        egui::Rect::from_min_max(egui::pos2(rect.min.x, rect.max.y - cl::space::S5), rect.max),
        radius,
        cl::scrim(150),
    );
    painter.text(
        egui::pos2(rect.center().x, rect.max.y - cl::space::S5 * 0.5),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(cl::Fluid::of(rect).fs_body_sm()),
        cl::ink::STRONG,
    );
    painter.rect_stroke(
        rect,
        radius,
        egui::Stroke::new(
            if picked { 2.0 } else { 1.0 },
            if picked {
                super::SPEC.tone.color()
            } else {
                cl::surface::HAIRLINE
            },
        ),
        egui::StrokeKind::Inside,
    );
    // A tile is painted, not built from a widget, so it carries no name of
    // its own. Say what it is: screen readers and the headless harness both
    // read this, and a control a machine cannot name is one a person cannot
    // either.
    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, picked, label));
    let tip = if picked {
        format!("{label} — this is what the card has now")
    } else {
        format!("Tap to put {} behind the words", label.to_lowercase())
    };
    let resp = cl::tip(resp, &tip);
    ui.add_space(cl::space::S2);
    resp
}

/// The shelves, in the operator's words.
///
/// [`Shelf::slug`] is the machine id — what the code calls the tier, and what
/// a proof script or pilot node keys on. [`Shelf::label`] is what gets drawn.
/// Same split `LibSection` makes in `library.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Shelf {
    #[default]
    Starters,
    Layouts,
    Combos,
    Pieces,
    Themes,
    Interactions,
}

impl Shelf {
    pub(crate) const ALL: [Shelf; 6] = [
        Shelf::Starters,
        Shelf::Layouts,
        Shelf::Combos,
        Shelf::Pieces,
        Shelf::Themes,
        Shelf::Interactions,
    ];

    /// Stable machine id — never drawn.
    pub(crate) fn slug(self) -> &'static str {
        match self {
            Shelf::Starters => "seeds",
            Shelf::Layouts => "templates",
            Shelf::Combos => "blocks",
            Shelf::Pieces => "elements",
            Shelf::Themes => "traits",
            Shelf::Interactions => "recipes",
        }
    }

    /// What the operator reads.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Shelf::Starters => "Starters",
            Shelf::Layouts => "Layouts",
            Shelf::Combos => "Combos",
            Shelf::Pieces => "Pieces",
            Shelf::Themes => "Themes",
            Shelf::Interactions => "Interactions",
        }
    }

    /// The second line on the chip — what the shelf holds, in three words.
    fn sub(self) -> &'static str {
        match self {
            Shelf::Starters => "ready to go",
            Shelf::Layouts => "whole cards",
            Shelf::Combos => "a few pieces together",
            Shelf::Pieces => "the small stuff",
            Shelf::Themes => "how it's dressed",
            Shelf::Interactions => "what a tap does",
        }
    }

    /// The tooltip law.
    fn tip(self) -> &'static str {
        match self {
            Shelf::Starters => "Whole cards, ready to go — tap one and make it yours",
            Shelf::Layouts => "Every card design you have, running live",
            Shelf::Combos => "A few pieces that keep turning up together",
            Shelf::Pieces => "The smallest things everything else is built from",
            Shelf::Themes => "How a card is dressed — pick one, stack more on top",
            Shelf::Interactions => "What happens when someone taps the card",
        }
    }
}

/// The overlay editor's tabs, in the operator's words.
///
/// **Words** is the REAL [`dpl_editor::Editor`] over the card's own
/// `slide.json` — editing a card edits its document, and edits save as you
/// type. The rest are honest placeholders: each names what will live there and
/// admits it is not built, which beats a control that does nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum CardTab {
    #[default]
    Words,
    Background,
    Movement,
    Quote,
    Picture,
    OnTap,
    Help,
}

impl CardTab {
    const ALL: [CardTab; 7] = [
        CardTab::Words,
        CardTab::Background,
        CardTab::Movement,
        CardTab::Quote,
        CardTab::Picture,
        CardTab::OnTap,
        CardTab::Help,
    ];

    fn label(self) -> &'static str {
        match self {
            CardTab::Words => "Words",
            CardTab::Background => "Background",
            CardTab::Movement => "Movement",
            CardTab::Quote => "A quote",
            CardTab::Picture => "Picture",
            CardTab::OnTap => "On tap",
            CardTab::Help => "Ask for help",
        }
    }

    /// Stable machine id — the pilot key. Never drawn.
    fn slug(self) -> &'static str {
        match self {
            CardTab::Words => "words",
            CardTab::Background => "background",
            CardTab::Movement => "movement",
            CardTab::Quote => "quote",
            CardTab::Picture => "picture",
            CardTab::OnTap => "on-tap",
            CardTab::Help => "help",
        }
    }

    fn tip(self) -> &'static str {
        match self {
            CardTab::Words => "What the card says — type and it saves itself",
            CardTab::Background => "What sits behind the words",
            CardTab::Movement => "How things arrive and leave",
            CardTab::Quote => "Someone else's words, kept with where they came from",
            CardTab::Picture => "An image or a video on the card",
            CardTab::OnTap => "What happens when someone taps it",
            CardTab::Help => "Hand this card and what you want to the AI",
        }
    }

    /// Does this tab get the live card beside it?
    ///
    /// Everywhere but Words, the card is a PREVIEW: change a colour, the card
    /// shows it. On Words the card is the CONTROL — you tap a line on the
    /// slide to select what to edit — so a second card beside it, only one of
    /// which answers a tap, would be worse than one. This is a deliberate
    /// deviation from the mock's uniform two-pane body, recorded in
    /// `docs-history-2026-10-12:docs/plans/author-overlay-live-pane.md` §8.
    fn shows_live_card(self) -> bool {
        !matches!(self, CardTab::Words)
    }

    /// What this tab will hold once built — shown INSTEAD of a fake control.
    fn not_yet(self) -> &'static str {
        match self {
            CardTab::Background => {
                "Two colours blended, something moving, or one flat colour — around forty to \
                 choose from, tapped straight onto the card."
            }
            CardTab::Movement => {
                "How things come in, get noticed, and go out — tapped on, and shown on the card \
                 beside you."
            }
            CardTab::Quote => {
                "The quote, who said it, and where it came from — kept together so it can be \
                 checked later."
            }
            CardTab::Picture => {
                "Paste a link or drop a file. A copy comes in with it, and your originals are \
                 never touched."
            }
            CardTab::OnTap => {
                "Reveal, flip, explode. Anything you already have drops straight in; anything \
                 new gets built properly on its own first."
            }
            CardTab::Help => {
                "What you are making plus this card, written up as one clear ask — with the \
                 pieces underneath built before the card that uses them."
            }
            CardTab::Words => "",
        }
    }
}

/// The open overlay editor — the real [`dpl_editor::Editor`] over the card's
/// `slide.json` (edits auto-save; the editor stays fabric-free per the layer
/// law) plus which tab is showing.
struct CardEditor {
    title: String,
    editor: dpl_editor::Editor,
    tab: CardTab,
    /// Which card's folder is open, so the strip can re-read it on close.
    dir: std::path::PathBuf,
    /// Which sort of picture the Picture tab is showing.
    pictures: crate::pictures::PictureKind,
    /// The shelf for [`Self::pictures`], read ONCE per kind.
    ///
    /// `pictures()` opens the artifact index — a database connection, not a
    /// lookup. Calling it per frame hung the UI thread hard enough that a
    /// proof run never reached its screenshot. Same law `containers.rs`
    /// follows: read on the gesture that changes it, never in the paint.
    shelf: Option<crate::pictures::Shelf>,
    /// The quote tab's field buffer, adopted from the document the first time
    /// the tab is opened. Held so typing does not write — and therefore does
    /// not snapshot undo — on every keystroke.
    quote: Option<[String; 3]>,
    /// The live pane's clock. Owned here rather than by a `DocSlide`,
    /// because the pane renders the editor's slide DIRECTLY — rebuilding a
    /// document each frame would restart the entrance on every keystroke.
    preview: dpl_slides::motion::Clock,
}

/// One card's worth of shelf metadata — owned, so painting can borrow the
/// surface mutably without fighting the list it is walking.
struct ShelfItem {
    /// The registry id. Machine-side; the card shows [`ShelfItem::label`].
    id: String,
    label: String,
    blurb: String,
    tip: String,
}

/// How long ago, in words a person uses. The folder's modified time is the
/// only fact available — anything more precise would imply a history this
/// surface does not keep.
fn coarse_when(when: Option<std::time::SystemTime>) -> String {
    let Some(when) = when else {
        return "unknown".to_string();
    };
    let Ok(ago) = std::time::SystemTime::now().duration_since(when) else {
        return "just now".to_string();
    };
    let hours = ago.as_secs() / 3600;
    match hours {
        0 => "in the last hour".to_string(),
        1..=23 => "today".to_string(),
        24..=47 => "yesterday".to_string(),
        48..=167 => "this week".to_string(),
        _ => "a while ago".to_string(),
    }
}

/// Which screen the authoring surface is showing.
///
/// The mock opens on a choice, not on the pieces — "Pick the lane first.
/// Everything after it … follows from this one choice." Every drill that
/// enters the content surface sets [`Screen::Content`] explicitly, so a proof
/// script lands where it always did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Screen {
    #[default]
    Dashboard,
    Content,
}

/// The authoring surface's transient state. Default-derivable so the hub
/// stays `#[derive(Default)]`.
#[derive(Default)]
pub(crate) struct AuthorSurface {
    /// The intent bar (charter §3b) — captured first, threads into the draft
    /// and (Pass E) the Ask-AI brief. Facts only; no fabric traffic until a
    /// mutation.
    intent: String,
    /// The last action's honest result line.
    status: String,
    /// Which shelf the carousel is showing.
    shelf: Shelf,
    /// Off, the carousel stops at the end of the shelf. On, it cycles so a
    /// sweep never dead-ends — the operator's "endless scrolling as a
    /// checkbox".
    endless: bool,
    /// Extra pages the running-on carousel has handed out. Grows on demand, so
    /// an untouched shelf costs nothing.
    pages: usize,
    /// A scroll target set by the arrows, applied on the next frame.
    scroll_to: Option<f32>,
    /// Where the carousel sits right now, read back each frame so the arrows
    /// know what "one page along" means.
    scroll_at: f32,
    /// Live template instances, built once. They carry interaction and clock
    /// state, so rebuilding per frame would reset every animation mid-sweep.
    demos: Vec<Box<dyn SlideTemplate>>,
    /// Seeds loaded from disk once, as real documents. `DocSlide` carries the
    /// clock, so each one animates on its own schedule exactly as it does on
    /// the Library's seeds wall — the same paint path, not a second one.
    seeds: Vec<(String, dpl_slides::document::DocSlide)>,
    /// Set after the first frame so the registries are read exactly once.
    loaded: bool,
    /// The open overlay editor, if a card is being edited.
    editing: Option<CardEditor>,
    /// Which screen the surface is on. The operator's third standing
    /// correction: declare intent FIRST, then reach the pieces.
    screen: Screen,
    /// Which sort of thing is being built into. `None` until the first load
    /// picks one — a strip labelled with nothing would be a lie.
    kind: Option<ContainerKind>,
    /// Every container of [`Self::kind`], read off disk when the kind
    /// changes rather than every frame.
    offered: Vec<Container>,
    /// Which of [`Self::offered`] the strip is labelled with.
    into: Option<usize>,
    /// The cards this sitting has started, oldest first. These live in
    /// `inbox/` and are NOT in the container yet — the strip says so.
    drafts: Vec<Draft>,
    /// What the strip actually took last frame, measured as it drew. Zero
    /// until it has drawn once; see [`AuthorSurface::strip_height`].
    strip_h: f32,
}

/// One card started in this sitting: what it is called, and where on disk it
/// actually is. The strip lists these; placing them into the container is a
/// real mutation and its own pass.
/// What a new card starts from.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Start {
    Blank,
    /// A component's id: its contract's first example.
    Layout(String),
    /// A seed's id: copied as the operator's own slide.
    Starter(String),
}

struct Draft {
    /// The inbox folder name — machine-side.
    slide: String,
    label: String,
    dir: std::path::PathBuf,
    /// The card's own document, drawn as a miniature in the strip. `None`
    /// when it will not load — the tile falls back to a labelled plate
    /// rather than showing a blank stage and pretending nothing is wrong.
    doc: Option<dpl_slides::document::DocSlide>,
}

/// Read a draft's document off disk. Called when the card is started and
/// again whenever its editor closes, because the editor saves as you type
/// and a strip still showing the old words would be lying.
fn load_draft_doc(dir: &std::path::Path) -> Option<dpl_slides::document::DocSlide> {
    dpl_doc::load(dir)
        .ok()
        .map(dpl_slides::document::DocSlide::new)
}

impl AuthorSurface {
    pub(crate) fn ui(&mut self, ui: &mut egui::Ui, fabric: &Option<dpl_fabric::Fabric>) {
        self.load_once();
        match self.screen {
            Screen::Dashboard => self.dashboard(ui),
            Screen::Content => self.content(ui, fabric),
        }
    }

    /// How many ingredients the operator has to build WITH. Every registry
    /// the Library already enumerates — no new count, no stored total.
    fn to_build_with(&self) -> usize {
        self.demos.len()
            + self.seeds.len()
            + dpl_slides::elements::elements().len()
            + dpl_slides::blocks::blocks().len()
            + cl::TRAITS.len()
    }

    /// Every container the operator has, across every kind, newest first.
    ///
    /// "Newest" is the folder's own modified time — a fact on disk, not a
    /// history this surface keeps. Anything richer would need a store, and an
    /// invented one is the padding this lane keeps refusing.
    fn everything_open(&self) -> Vec<(Container, Option<std::time::SystemTime>)> {
        let root = dpl_engine_capture::content_root();
        let mut all: Vec<(Container, Option<std::time::SystemTime>)> = ContainerKind::ALL
            .into_iter()
            .flat_map(|kind| containers(kind, &root))
            .map(|c| {
                let when = std::fs::metadata(&c.dir).and_then(|m| m.modified()).ok();
                (c, when)
            })
            .collect();
        // Newest first. `None` (no readable mtime) sorts last, exactly as the
        // hand-rolled descending comparator did.
        all.sort_by_key(|(_, when)| std::cmp::Reverse(*when));
        all
    }

    /// The dashboard — declare what you are doing before reaching the pieces.
    fn dashboard(&mut self, ui: &mut egui::Ui) {
        let fluid = cl::Fluid::of(ui.max_rect());
        let open = self.everything_open();
        let mut go: Option<Option<usize>> = None;

        // A legibility scrim under the whole screen.
        //
        // The content surface fills its panel and hides the backdrop by
        // sheer volume; the dashboard is deliberately sparse, so the
        // artwork came straight through and the lane footers were barely
        // readable. This is the same legibility diffusion the media painter
        // lays under a picture before words sit on it — the vocabulary
        // already had the answer.
        ui.painter()
            .rect_filled(ui.max_rect(), cl::RADIUS, cl::scrim(196));
        ui.add_space(cl::space::S2);

        ui.label(
            RichText::new("DEVPULSE · AUTHOR")
                .size(fluid.fs_eyebrow())
                .color(cl::ink::MUTED),
        );
        ui.label(
            RichText::new("What are you working on today?")
                .heading()
                .color(cl::ink::STRONG),
        );
        ui.label(
            RichText::new(
                "Pick the lane first. Everything after it — what you see, what the buttons \
                 say, where the file lands — follows from this one choice.",
            )
            .small()
            .color(cl::ink::MUTED),
        );
        ui.add_space(cl::space::S5);

        // Three lanes. Every number is read from a census that already exists;
        // none is stored and none is invented.
        ui.horizontal_wrapped(|ui| {
            let lanes: [(Tone, &str, &str, String); 3] = [
                (
                    Tone::Cyan,
                    "Make something",
                    "Build cards into a lesson, guide, episode or deck. Pick one you \
                     already have, or start fresh.",
                    format!("{} open", open.len()),
                ),
                (
                    Tone::Amber,
                    "Pick up where I left off",
                    "Straight back into the last thing you touched, exactly as you left \
                     it.",
                    match open.first() {
                        Some((c, _)) => format!("last · {}", c.label),
                        None => "nothing yet".to_string(),
                    },
                ),
                (
                    Tone::Magenta,
                    "Build a new piece",
                    "Add to what you can build WITH — a piece, a combo, a theme, a whole \
                     layout. One kit, mixed and matched.",
                    format!("{} to build with", self.to_build_with()),
                ),
            ];
            for (ix, (tone, title, blurb, foot)) in lanes.into_iter().enumerate() {
                let w = (ui.available_width() / 3.0 - cl::space::S4).max(fluid.panel_width() * 0.6);
                let hit = cl::tip(
                    stacked_control(ui, title, blurb, &foot, false, tone, w),
                    match ix {
                        0 => "Choose what to build into, then start cards",
                        1 => "Straight back into the last thing you touched",
                        _ => "Everything you can build with, on one wall",
                    },
                );
                ui.pilot_track(
                    &hit,
                    NodeMetadata::new(
                        PilotId::new("workbench.author.lane").child(&PilotKey::new(match ix {
                            0 => "make",
                            1 => "resume",
                            _ => "build",
                        })),
                        egui_pilot_ai::protocol::Role::Button,
                        title,
                    ),
                );
                if hit.clicked() {
                    // Lane 3 belongs to M9's platform lane, which does not
                    // exist. The Library IS "what you can build with", so it
                    // goes there rather than nowhere.
                    go = Some(match ix {
                        1 => Some(0),
                        _ => None,
                    });
                }
                ui.add_space(cl::space::S4);
            }
        });

        ui.add_space(cl::space::S5);
        ui.label(
            RichText::new("OPEN NOW")
                .size(fluid.fs_eyebrow())
                .color(cl::ink::MUTED),
        );
        ui.label(
            RichText::new("every kind you have — new kinds can be added any time")
                .size(fluid.fs_body_sm())
                .color(cl::ink::SUBTLE),
        );
        ui.add_space(cl::space::S2);

        if open.is_empty() {
            ui.label(
                RichText::new(
                    "Nothing open yet. Make something, and whatever you build into shows \
                     up here.",
                )
                .color(cl::ink::MUTED),
            );
        }
        ui.horizontal_wrapped(|ui| {
            for (ix, (c, when)) in open.iter().enumerate() {
                let hit = cl::tip(
                    control(
                        ui,
                        &format!(
                            "{} · {} · {} {} · {}",
                            c.kind.label(),
                            c.label,
                            c.count,
                            c.kind.unit(),
                            coarse_when(*when),
                        ),
                        false,
                        c.kind.tone(),
                        Weight::Chrome,
                    ),
                    "Go straight into this one",
                );
                if hit.clicked() {
                    go = Some(Some(ix));
                }
                ui.add_space(cl::space::S3);
            }
        });

        if let Some(which) = go {
            if let Some(ix) = which {
                if let Some((c, _)) = open.get(ix) {
                    self.choose_kind(c.kind);
                    self.into = self.offered.iter().position(|o| o.slug == c.slug);
                }
            }
            self.screen = Screen::Content;
        }
    }

    /// The content surface — everything downstream of the choice.
    fn content(&mut self, ui: &mut egui::Ui, fabric: &Option<dpl_fabric::Fabric>) {
        ui.horizontal(|ui| {
            if cl::tip(
                ui.label(RichText::new("← Back").strong().color(cl::ink::MUTED))
                    .interact(egui::Sense::click()),
                "Back to choosing what you are working on",
            )
            .clicked()
            {
                self.screen = Screen::Dashboard;
            }
        });
        ui.label(
            RichText::new("AUTHOR — turn an intention into a card")
                .heading()
                .color(cl::ink::STRONG),
        );
        ui.label(
            RichText::new(
                "Say what you want, then sweep the shelf below and tap what you want to \
                 start from. Everything you see here is drawn for real, not a picture of it.",
            )
            .small()
            .color(cl::ink::MUTED),
        );
        ui.add_space(cl::space::S3);

        // Intent bar — charter §3b. Captured, editable, no signal yet.
        ui.label(
            RichText::new("What do you want to make?")
                .strong()
                .color(cl::ink::BASE),
        );
        cl::tip(
            ui.add(
                egui::TextEdit::singleline(&mut self.intent)
                    .hint_text("e.g. a title card that says “DevPULSE weekly”")
                    .desired_width(f32::INFINITY),
            ),
            "What you're after. It becomes the card's words, and later the wording sent to AI.",
        );
        ui.add_space(cl::space::S3);

        self.load_once();
        self.container_bar(ui);
        self.shelf_picker(ui);
        // The strip is drawn last but reserved for FIRST — otherwise a tall
        // card pushes the band off the bottom edge exactly as it once pushed
        // its own captions off.
        let strip_h = self.strip_height(&cl::Fluid::of(ui.max_rect()));
        let picked = self.carousel(ui, strip_h);

        ui.add_space(cl::space::S3);
        let seed = cl::tip(
            control(ui, "+ Start a blank card", false, Tone::Amber, Weight::Hero),
            "Make a card with nothing on it but your words, and add to it from there.",
        );

        // ONE authoring path: a tapped card and the blank button both land here,
        // so a card minted by a tap is the same real document as any other.
        let request = picked.or_else(|| seed.clicked().then(|| ("blank".to_string(), None)));
        if let Some((from, label)) = request {
            // What was tapped is what the card starts from (LS2): a Layout
            // fills its component's example, a Starter is copied as your own.
            let start = match self.shelf {
                _ if from == "blank" => Start::Blank,
                Shelf::Layouts => Start::Layout(from),
                Shelf::Starters => Start::Starter(from),
                _ => Start::Blank,
            };
            self.start_card(start, label, fabric);
        }

        if !self.status.is_empty() {
            ui.add_space(cl::space::S2);
            ui.label(RichText::new(&self.status).small().color(cl::ink::BASE));
        }

        // The band sits along the bottom, on the room reserved for it before
        // the carousel sized itself.
        let space = ui.available_height() - self.strip_height(&cl::Fluid::of(ui.max_rect()));
        if space > 0.0 {
            ui.add_space(space);
        }
        if self.filmstrip(ui) {
            // The strip's own tile is the same authoring path as everything
            // else — not a second way to make a card.
            self.start_card(Start::Blank, None, fabric);
        }

        // Last, so the scrim sits over everything drawn above it.
        self.editor_modal(ui);
    }

    /// Put one of this sitting's cards into the loaded container.
    ///
    /// `dpl_doc::place_slide` does the mutation and does it carefully: it
    /// COPIES (the inbox draft survives), refuses a slide that does not
    /// validate, refuses to create an episode, never clobbers a name, and
    /// rolls the episode back if the result would be invalid. So this is a
    /// gesture over a safe primitive, not a new mutation.
    ///
    /// On success the draft leaves the strip — it is in the container now —
    /// and the count is RE-READ from disk rather than incremented, so the
    /// number on screen is the number in the folder. On failure the card
    /// stays and the reason is shown verbatim: an operator who believes their
    /// work is filed when it is not is the worst outcome this surface can
    /// produce.
    /// `root` is passed rather than read from `content_root()`, which is a
    /// process-wide `OnceLock` and therefore un-pointable from a test. Taking
    /// it as an argument is both better design and what makes this path
    /// provable.
    fn place_draft(&mut self, ix: usize, chapter: &str, root: &std::path::Path) {
        let Some(container) = self.container().cloned() else {
            return;
        };
        let Some(draft) = self.drafts.get(ix) else {
            return;
        };
        let Some((show, episode)) = container.slug.split_once('/') else {
            self.status = "that container's name is not a show and an episode".into();
            return;
        };
        match dpl_doc::place_slide(root, &draft.dir, show, episode, chapter, None) {
            Ok(placed) => {
                self.status = format!(
                    "put “{}” into {} — it is now {}/{}",
                    draft.label, container.label, placed.chapter, placed.slide
                );
                self.drafts.remove(ix);
                // Re-read, never increment: the count is a fact about the
                // folder, not a tally the surface keeps.
                self.offered = containers(container.kind, root);
                self.into = self
                    .offered
                    .iter()
                    .position(|c| c.slug == container.slug)
                    .or(self.into);
            }
            // Verbatim. `place_slide` already says why in plain-enough words
            // ("slide is not finished…", "chapter … not in episode"), and
            // rewording it here would only lose detail.
            Err(why) => self.status = format!("could not put that card in: {why}"),
        }
    }

    /// Start a card: write the document, remember it, open its editor, then
    /// publish the fact. The ONE path — every control that starts a card
    /// comes through here.
    fn start_card(
        &mut self,
        start: Start,
        label: Option<String>,
        fabric: &Option<dpl_fabric::Fabric>,
    ) {
        match self.mint_draft(&start) {
            Ok((slide, dir)) => {
                self.status = match &label {
                    Some(l) => format!("started “{l}” — saved in your library"),
                    None => "started a blank card — saved in your library".to_string(),
                };
                // Straight into the editor. Starting a card and then having
                // to go find it is exactly the friction this surface exists
                // to remove, and the document is already on disk — so the
                // editor opens on a real file, not a buffer hoping to be one.
                let title = label.unwrap_or_else(|| "New card".to_string());
                // Remembered BEFORE the editor opens, so the strip shows it
                // whether or not the editor could be opened on it.
                self.drafts.push(Draft {
                    slide: slide.clone(),
                    label: title.clone(),
                    doc: load_draft_doc(&dir),
                    dir: dir.clone(),
                });
                self.open_editor(&dir, title);
                // Emit-with-mutation: the fact is published AFTER the write
                // succeeds, never before. `CardStarted` carries only the draft
                // id — show/episode are not facts yet at this point in the
                // flow, and the old signal carried them as empty strings,
                // which is a lie shaped like data. `CardSaved` carries them
                // once placing has made them true (Studio's `SlidePlaced`).
                if let Some(f) = fabric {
                    f.publish_build(dpl_fabric::BuildEvent::CardStarted { slide });
                }
            }
            Err(e) => self.status = format!("could not start a card: {e} (nothing was written)"),
        }
    }

    /// Mint a card and open its editor — the `--author-editor` proof seam.
    ///
    /// The overlay is reachable only by tapping, so without this the surface
    /// this pass adds could not be photographed on the real exe. Point
    /// `--content-root` at a scratch dir and it writes nowhere near real work.
    pub(crate) fn open_editor_drill(&mut self) {
        // The same path a tap takes — a drill that minted differently would
        // prove a code path the operator never walks.
        self.start_card(Start::Blank, None, &None);
        self.screen = Screen::Content;
    }

    /// Open the overlay editor on one TAB by its slug — the
    /// `--card-tab <slug>` proof seam. Words is the default, so no tab but
    /// that one could be photographed on the real exe before this.
    ///
    /// Unknown slugs leave the tab alone rather than guessing.
    pub(crate) fn card_tab_drill(&mut self, slug: &str) {
        self.screen = Screen::Content;
        if self.editing.is_none() {
            // Re-open the newest card in the inbox rather than minting another
            // one. A drill that shows a tab should not need a fresh card each
            // time, and re-opening is what a person does anyway.
            match newest_draft() {
                Some(dir) => {
                    let label = dir
                        .file_name()
                        .map(|n| pretty(&n.to_string_lossy()))
                        .unwrap_or_else(|| "New card".to_string());
                    self.open_editor(&dir, label);
                }
                None => self.open_editor_drill(),
            }
        }
        if let (Some(ed), Some(tab)) = (
            self.editing.as_mut(),
            CardTab::ALL.into_iter().find(|t| t.slug() == slug),
        ) {
            ed.tab = tab;
        }
    }

    /// Start `n` cards WITHOUT opening the editor — the `--author-draft <n>`
    /// proof seam, so the filmstrip can be photographed with something in it.
    /// Same mint, then the overlay is dismissed so the band is visible.
    pub(crate) fn draft_drill(&mut self, n: usize) {
        for _ in 0..n {
            self.start_card(Start::Blank, None, &None);
        }
        self.editing = None;
        self.screen = Screen::Content;
    }

    /// Show one kind's shelf of containers — the `--author-container <kind>`
    /// proof seam. An unknown slug is ignored rather than guessed at.
    pub(crate) fn container_drill(&mut self, slug: &str) {
        if let Some(kind) = ContainerKind::ALL.into_iter().find(|k| k.slug() == slug) {
            self.load_once();
            self.choose_kind(kind);
        }
        // Every seam enters the CONTENT surface. The dashboard is where a
        // person starts; a proof script has already made its choice.
        self.screen = Screen::Content;
    }

    /// Open the overlay editor on a card's own folder.
    fn open_editor(&mut self, dir: &std::path::Path, title: String) {
        match dpl_editor::Editor::open(dir) {
            Ok(editor) => {
                self.editing = Some(CardEditor {
                    title,
                    editor,
                    tab: CardTab::default(),
                    dir: dir.to_path_buf(),
                    quote: None,
                    pictures: crate::pictures::PictureKind::default(),
                    shelf: None,
                    preview: dpl_slides::motion::Clock::default(),
                })
            }
            Err(e) => {
                self.status = format!("the card was saved, but its editor would not open: {e}")
            }
        }
    }

    /// The overlay editor — a scrim over the surface, tabs across the top.
    ///
    /// Words is the real editor over the card's `slide.json`; the rest name
    /// what they will hold and admit they are not built.
    fn editor_modal(&mut self, ui: &mut egui::Ui) {
        let Some(ed) = self.editing.as_mut() else {
            return;
        };
        let tone = super::SPEC.tone;
        let mut close = false;

        egui::Modal::new(egui::Id::new("author.card-editor"))
            .backdrop_color(egui::Color32::from_black_alpha(184))
            .frame(
                egui::Frame::new()
                    .fill(cl::surface::RAISED)
                    .stroke(egui::Stroke::new(1.0, tone.color()))
                    .corner_radius(egui::CornerRadius::same(cl::RADIUS_LG))
                    .inner_margin(egui::Margin::same(16)),
            )
            .show(ui.ctx(), |ui| {
                let room = ui.ctx().content_rect();
                let fluid = cl::Fluid::of(room);
                // Wide enough for the editor to breathe, never wider than the
                // window it sits in.
                ui.set_width((fluid.panel_width() * 2.0).min(room.width() * 0.8));

                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(&ed.title)
                            .size(fluid.fs_h4())
                            .strong()
                            .color(cl::ink::STRONG),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if cl::tip(
                            control(ui, "Done", false, tone, Weight::Hero),
                            "Close this — your card is already saved.",
                        )
                        .clicked()
                        {
                            close = true;
                        }
                    });
                });
                ui.add_space(cl::space::S2);

                ui.separator();

                // R1's shape: a rail down the left, the tab's controls in the
                // middle, the card itself live on the right.
                //
                // Explicit rects rather than nested layouts — the same idiom
                // the carousel uses, and for the same reason: a band whose
                // parts must line up is easier to reason about as geometry
                // than as a stack of layout negotiations.
                let rail_w = fluid.panel_width() * 0.44;
                let body_h = (room.height() * 0.5).max(fluid.panel_width());
                // The rail only goes vertical when it can afford a column
                // beside two more. Below that it wraps across the top, which
                // is the degrade R1 makes at 760px.
                let vertical_rail = ui.available_width() > rail_w * 4.0;
                if !vertical_rail {
                    tab_rail_wrapped(ui, ed, tone);
                    ui.add_space(cl::space::S2);
                }

                // The card only earns a pane where it is a PREVIEW. On Words
                // it is the control — you tap a line on the slide to select
                // it — and two cards side by side, only one of which answers
                // a tap, is worse than one.
                let live = ed.tab.shows_live_card();
                let (band, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), body_h),
                    egui::Sense::hover(),
                );
                let gap = cl::space::S3;
                let mut x = band.min.x;
                if vertical_rail {
                    let rail = egui::Rect::from_min_size(band.min, egui::vec2(rail_w, body_h));
                    // `new_child`, not `scope_builder`: the band already
                    // claimed its height, and a scope would claim more on top
                    // — which walked the footer up into the band.
                    let mut rail_ui = ui.new_child(egui::UiBuilder::new().max_rect(rail));
                    rail_ui.set_clip_rect(rail.intersect(ui.clip_rect()));
                    tab_rail(&mut rail_ui, ed, tone);
                    x = rail.max.x + gap;
                }
                let live_w = if live {
                    ((band.max.x - x) * 0.4).min(fluid.panel_width())
                } else {
                    0.0
                };
                let controls = egui::Rect::from_min_max(
                    egui::pos2(x, band.min.y),
                    egui::pos2(
                        band.max.x - live_w - if live { gap } else { 0.0 },
                        band.max.y,
                    ),
                );
                let mut controls_ui = ui.new_child(egui::UiBuilder::new().max_rect(controls));
                controls_ui.set_clip_rect(controls.intersect(ui.clip_rect()));
                {
                    let ui = &mut controls_ui;
                    egui::ScrollArea::vertical()
                        .id_salt("author.card-editor.controls")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.set_clip_rect(controls.intersect(ui.clip_rect()));
                            match ed.tab {
                                CardTab::Words => ed.editor.ui(ui),
                                CardTab::Background => background_tab(ui, ed),
                                CardTab::Movement => movement_tab(ui, ed),
                                CardTab::Quote => quote_tab(ui, ed),
                                CardTab::Picture => picture_tab(ui, ed),
                                other => {
                                    ui.add_space(cl::space::S4);
                                    ui.label(
                                        RichText::new("Not built yet.")
                                            .size(fluid.fs_body_lg())
                                            .strong()
                                            .color(cl::ink::STRONG),
                                    );
                                    ui.add_space(cl::space::S2);
                                    ui.label(RichText::new(other.not_yet()).color(cl::ink::MUTED));
                                }
                            }
                        });
                }
                if live {
                    let pane = egui::Rect::from_min_max(
                        egui::pos2(band.max.x - live_w, band.min.y),
                        band.max,
                    );
                    let mut pane_ui = ui.new_child(egui::UiBuilder::new().max_rect(pane));
                    pane_ui.set_clip_rect(pane.intersect(ui.clip_rect()));
                    live_pane(&mut pane_ui, ed);
                }

                ui.separator();
                ui.label(
                    RichText::new("Everything you type here saves itself.")
                        .small()
                        .color(cl::ink::SUBTLE),
                );
            });

        if close {
            // The editor writes as you type, so the strip's copy of this
            // card is stale the moment it closes. Re-read it.
            if let Some(ed) = self.editing.take() {
                if let Some(draft) = self.drafts.iter_mut().find(|d| d.dir == ed.dir) {
                    draft.doc = load_draft_doc(&draft.dir);
                }
            }
        }
    }

    /// The shelf picker — tap a shelf, the carousel below swaps to it.
    fn shelf_picker(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for shelf in Shelf::ALL {
                let active = self.shelf == shelf;
                // The shelf NAME only — its one-line "what's on it" belongs in
                // the tooltip and in the carousel header, not doubled across a
                // row that then wraps onto two lines.
                let chip = cl::tip(
                    control(ui, shelf.label(), active, super::SPEC.tone, Weight::Chrome),
                    shelf.tip(),
                );
                ui.pilot_track(
                    &chip,
                    // Keyed on the slug so proof scripts survive a rewording.
                    NodeMetadata::new(
                        PilotId::new("workbench.author.shelf").child(&PilotKey::new(shelf.slug())),
                        egui_pilot_ai::protocol::Role::Tab,
                        shelf.label(),
                    )
                    .value(if active { "active" } else { "available" })
                    .selected(active),
                );
                if chip.clicked() && !active {
                    self.shelf = shelf;
                    // A new shelf starts at its beginning, not wherever the
                    // last one happened to be scrolled to.
                    self.pages = 0;
                    self.scroll_to = Some(0.0);
                }
                ui.add_space(cl::space::S3);
            }
        });
        ui.add_space(cl::space::S2);
    }

    /// Load the containers of one kind and land on the first of them.
    ///
    /// Reads the disk, so it runs on a kind change and at first load — never
    /// per frame.
    fn choose_kind(&mut self, kind: ContainerKind) {
        self.kind = Some(kind);
        self.offered = containers(kind, &dpl_engine_capture::content_root());
        self.into = (!self.offered.is_empty()).then_some(0);
    }

    /// The container currently being built into.
    fn container(&self) -> Option<&Container> {
        self.into.and_then(|ix| self.offered.get(ix))
    }

    /// "Building into …" — the kind, then which one.
    ///
    /// Both pickers are drawn from [`ContainerKind::ALL`] and the disk census; neither
    /// is a hand-list. A kind with nothing behind it says why in its own
    /// words instead of offering an empty menu.
    fn container_bar(&mut self, ui: &mut egui::Ui) {
        let fluid = cl::Fluid::of(ui.max_rect());
        let kind = self.kind.unwrap_or(ContainerKind::Episode);
        let mut want = None;

        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("Building into")
                    .size(fluid.fs_body_sm())
                    .color(cl::ink::MUTED),
            );
            ui.add_space(cl::space::S2);

            let kind_box = egui::ComboBox::from_id_salt("author.kind")
                .selected_text(
                    RichText::new(kind.label())
                        .strong()
                        .color(kind.tone().color()),
                )
                .show_ui(ui, |ui| {
                    for k in ContainerKind::ALL {
                        let on = k == kind;
                        let mut picked = on;
                        let item = cl::tip(
                            ui.toggle_value(
                                &mut picked,
                                RichText::new(format!("{} — {}", k.label(), k.blurb()))
                                    .color(if on { k.tone().color() } else { cl::ink::BASE }),
                            ),
                            k.tip(),
                        );
                        if item.clicked() && k != kind {
                            want = Some(k);
                        }
                    }
                });
            cl::tip(kind_box.response, "What sort of thing you're building into");

            ui.add_space(cl::space::S3);

            // Which one — only ever the containers that really exist.
            if self.offered.is_empty() {
                // Two different emptinesses, and they must not read alike:
                // a kind with no home at all, versus a real kind you simply
                // have none of yet.
                let why = if kind.is_stored() {
                    format!(
                        "No {} yet — make one and it shows up here.",
                        kind.label().to_lowercase()
                    )
                } else {
                    kind.not_stored_yet().unwrap_or_default().to_string()
                };
                cl::tip(
                    ui.label(
                        RichText::new(&why)
                            .size(fluid.fs_body_sm())
                            .color(cl::ink::MUTED),
                    ),
                    "Nothing to build into under this heading yet",
                );
            } else {
                let current = self
                    .container()
                    .map(|c| c.label.clone())
                    .unwrap_or_default();
                let which = egui::ComboBox::from_id_salt("author.container")
                    .selected_text(RichText::new(&current).strong().color(cl::ink::STRONG))
                    .show_ui(ui, |ui| {
                        for (ix, c) in self.offered.iter().enumerate() {
                            let mut picked = self.into == Some(ix);
                            let item = ui.toggle_value(
                                &mut picked,
                                RichText::new(format!(
                                    "{}  ({} {})",
                                    c.label,
                                    c.count,
                                    kind.unit()
                                ))
                                .color(cl::ink::BASE),
                            );
                            if item.clicked() {
                                self.into = Some(ix);
                            }
                        }
                    });
                cl::tip(which.response, "Which one you're adding to");
            }
        });

        if let Some(k) = want {
            self.choose_kind(k);
        }
        ui.add_space(cl::space::S3);
    }

    /// How much room the strip needs, so the carousel can subtract it BEFORE
    /// it proposes a card size. The band is the last thing drawn but the
    /// first thing accounted for; otherwise a tall card pushes it off the
    /// bottom edge exactly as it once pushed its own captions off.
    ///
    /// The head wraps at narrow widths, so its height is not something a sum
    /// of tokens can predict. Rather than guess a number big enough to be
    /// safe — which is a magic px by another name — the band MEASURES itself
    /// as it draws and the next frame reserves exactly that. The seed below
    /// is only ever used for frame one, and it converges immediately because
    /// the strip's height does not depend on what was reserved for it.
    fn strip_height(&self, fluid: &cl::Fluid) -> f32 {
        if self.strip_h > 0.0 {
            return self.strip_h;
        }
        // Frame one: hairline gap + one head line + a film card + padding.
        cl::space::S3 + fluid.fs_body_lg() + cl::space::S2 + fluid.ctl_thumb_h() + cl::space::S4
    }

    /// The filmstrip — what you have started, labelled by what it is for.
    ///
    /// The head is drawn from the loaded container, never a constant. What it
    /// lists are this sitting's drafts, which live in `inbox/` and are NOT in
    /// the container yet; the wording says exactly that, because claiming a
    /// placement that has not happened is the one thing this band must not do.
    fn filmstrip(&mut self, ui: &mut egui::Ui) -> bool {
        let fluid = cl::Fluid::of(ui.max_rect());
        let Some(kind) = self.kind else {
            return false;
        };
        let mut start_one = false;
        // Which chapter the operator asked to put this sitting's cards into.
        let mut put_in: Option<String> = None;
        // Where the band begins, so it can report what it actually took.
        let top = ui.cursor().top();

        // A hairline is what separates the band from the surface it serves —
        // the strip is a shelf, not another card competing for attention.
        let seam = ui.available_rect_before_wrap();
        ui.painter().hline(
            seam.x_range(),
            seam.top(),
            egui::Stroke::new(1.0, cl::surface::HAIRLINE),
        );
        ui.add_space(cl::space::S3);

        ui.horizontal_wrapped(|ui| {
            // The kind pill — the strip's one piece of colour, so which sort
            // of thing you are building into reads at a glance.
            cl::tip(
                ui.label(
                    RichText::new(format!(" {} ", kind.label().to_uppercase()))
                        .size(fluid.fs_eyebrow())
                        .strong()
                        .color(kind.tone().color()),
                ),
                kind.tip(),
            );
            ui.add_space(cl::space::S2);

            match self.container() {
                Some(c) => {
                    ui.label(
                        RichText::new(&c.label)
                            .size(fluid.fs_body_lg())
                            .strong()
                            .color(cl::ink::STRONG),
                    );
                    ui.label(
                        RichText::new(format!("· already has {} {}", c.count, kind.unit()))
                            .size(fluid.fs_body_sm())
                            .color(cl::ink::MUTED),
                    );
                }
                None => {
                    ui.label(
                        RichText::new("Nothing chosen to build into yet")
                            .size(fluid.fs_body_lg())
                            .color(cl::ink::MUTED),
                    );
                }
            }

            ui.add_space(cl::space::S3);
            if !self.drafts.is_empty() {
                cl::tip(
                    ui.label(
                        RichText::new(format!(
                            "{} started here, not put in yet",
                            self.drafts.len()
                        ))
                        .size(fluid.fs_body_sm())
                        .color(Tone::Amber.color()),
                    ),
                    "Cards you started this sitting. They are saved in your inbox until \
                     you put them in.",
                );
                ui.add_space(cl::space::S3);
                // The gesture the strip has been promising. Only an episode
                // holds slide folders; every other kind says why instead of
                // offering a button that cannot work.
                match kind.cannot_take_a_card() {
                    Some(why) => {
                        cl::tip(
                            ui.label(
                                RichText::new("can't put cards in here")
                                    .size(fluid.fs_body_sm())
                                    .color(cl::ink::SUBTLE),
                            ),
                            why,
                        );
                    }
                    None => {
                        let chapters = self.container().map(|c| c.chapters.clone());
                        match chapters.as_deref() {
                            // Nothing to put them in — an episode with no
                            // chapters cannot receive a card, and saying so
                            // beats a button that always fails.
                            Some([]) | None => {
                                cl::tip(
                                    ui.label(
                                        RichText::new("no chapters to put them in")
                                            .size(fluid.fs_body_sm())
                                            .color(cl::ink::SUBTLE),
                                    ),
                                    "This episode has no chapters yet, and a card goes into \
                                     a chapter.",
                                );
                            }
                            // One chapter is not a decision.
                            Some([(id, _)]) => {
                                if cl::tip(
                                    control(
                                        ui,
                                        "Put them in",
                                        false,
                                        super::SPEC.tone,
                                        Weight::Hero,
                                    ),
                                    "Copy every card you started into this one. Your \
                                     originals stay where they are.",
                                )
                                .clicked()
                                {
                                    put_in = Some(id.clone());
                                }
                            }
                            // More than one IS a decision, so ask.
                            Some(many) => {
                                ui.label(
                                    RichText::new("Put them in")
                                        .size(fluid.fs_body_sm())
                                        .color(cl::ink::MUTED),
                                );
                                for (id, title) in many {
                                    if cl::tip(
                                        control(ui, title, false, super::SPEC.tone, Weight::Hero),
                                        "Copy every card you started into this chapter",
                                    )
                                    .clicked()
                                    {
                                        put_in = Some(id.clone());
                                    }
                                    ui.add_space(cl::space::S2);
                                }
                            }
                        }
                    }
                }
            }
        });
        ui.add_space(cl::space::S2);

        // The film itself — a row of what has been started, then a tile that
        // starts one more.
        // `ctl-thumb-*` IS the authored size for a thumbnail, and a strip item
        // is a thumbnail — half again as wide so the tap target clears
        // `tap-min`, and no larger. A tile sized like a hero card starves the
        // carousel it sits under: at 1100x700 that cost the sweep two thirds
        // of its height, which is a worse trade than a smaller preview.
        let card_w = fluid.ctl_thumb_w() * 1.5;
        let card_h = card_w * 9.0 / 16.0 + fluid.fs_body_sm() + cl::space::S2;
        egui::ScrollArea::horizontal()
            .id_salt("author.filmstrip")
            .max_height(card_h)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if self.drafts.is_empty() {
                        ui.label(
                            RichText::new(format!(
                                "Tap something above to drop your first {} in here.",
                                kind.unit_one()
                            ))
                            .size(fluid.fs_body())
                            .color(cl::ink::MUTED),
                        );
                    }
                    let mut reopen = None;
                    for (ix, draft) in self.drafts.iter_mut().enumerate() {
                        let resp = film_tile(
                            ui,
                            card_w,
                            card_h,
                            &draft.label,
                            kind.tone(),
                            &mut draft.doc,
                        );
                        let tip = format!(
                            "Open “{}” again — it is saved as inbox/{}",
                            draft.label, draft.slide
                        );
                        if cl::tip(resp, &tip).clicked() {
                            reopen = Some(ix);
                        }
                        ui.add_space(cl::space::S2);
                    }
                    // The new-card tile has no card behind it yet, so it is
                    // the one plate that stays a plate.
                    let plus = film_tile(ui, card_h, card_h, "+", Tone::Amber, &mut None);
                    if cl::tip(plus, "Start one more card").clicked() {
                        start_one = true;
                    }
                    if let Some(ix) = reopen {
                        let (dir, label) = {
                            let d = &self.drafts[ix];
                            (d.dir.clone(), d.label.clone())
                        };
                        self.open_editor(&dir, label);
                    }
                });
            });
        if let Some(chapter) = put_in {
            // Oldest first, and from the front each time, because a successful
            // place removes the draft it just filed.
            while !self.drafts.is_empty() {
                let before = self.drafts.len();
                self.place_draft(0, &chapter, &dpl_engine_capture::content_root());
                if self.drafts.len() == before {
                    // It refused. Stop rather than retrying the same card
                    // forever — the reason is already in the status line.
                    break;
                }
            }
        }

        // Measured, not guessed — the next frame reserves exactly this.
        self.strip_h = (ui.cursor().top() - top).max(0.0);
        start_one
    }

    /// ONE carousel: big cards, swept sideways. Returns what was tapped.
    ///
    /// Three cards across whatever the window, so the targets stay thumb-sized
    /// on the 75" prop and the layout never needs a magic width.
    fn carousel(
        &mut self,
        ui: &mut egui::Ui,
        reserve_below: f32,
    ) -> Option<(String, Option<String>)> {
        let fluid = cl::Fluid::of(ui.max_rect());
        let items = self.shelf_items();
        let mut picked = None;

        // Header: what this shelf is, how much of it there is, and the toggle.
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{} — {}", self.shelf.label(), self.shelf.sub()))
                    .size(fluid.fs_h4())
                    .strong()
                    .color(cl::ink::STRONG),
            );
            ui.add_space(cl::space::S3);
            ui.label(
                RichText::new(if items.is_empty() {
                    "nothing here yet".to_string()
                } else if self.endless {
                    format!("{} · keeps going", items.len())
                } else {
                    format!("{} in all", items.len())
                })
                .small()
                .color(cl::ink::MUTED),
            );
            ui.add_space(cl::space::S4);
            cl::tip(
                ui.checkbox(&mut self.endless, "keeps going"),
                "On, sweeping never dead-ends — the shelf cycles round. Off, it stops at the end.",
            );
        });
        ui.add_space(cl::space::S2);

        if items.is_empty() {
            self.empty_shelf(ui, &fluid);
            return None;
        }

        // Geometry, decided ONCE against the room we actually have. Three cards
        // across whatever the window, so the targets stay thumb-sized on the
        // 75" prop without a magic width anywhere.
        let gap = cl::space::S3;
        let arrow_w = fluid.sp_7();
        let avail = ui.available_width();
        let lane_w = (avail - (arrow_w + gap) * 2.0).max(arrow_w);
        // As many cards as fit COMFORTABLY, not a fixed three. A card holds a
        // whole slide drawn at real size, and the type ramp has a floor — squeeze
        // it below a panel's width and a headline clamps to something enormous
        // and crops. `panel_width` is the token for "a readable column of
        // content", so it is the honest minimum here too.
        let across = (lane_w / fluid.panel_width()).floor().clamp(1.0, 4.0);
        let card_w = ((lane_w - gap * (across - 1.0)) / across).max(arrow_w);
        // Width proposes, HEIGHT disposes. A tall card on a short window pushed
        // its own captions — and everything under the carousel — off the bottom
        // edge. Whichever axis is tighter decides the card, so the band always
        // fits the room it was given.
        let room_below = fluid.fs_body_lg() + cl::space::S6 + reserve_below;
        let room = (ui.available_height() - room_below).max(arrow_w * 2.0);
        // Room under the stage for a name AND its line, not one on top of the
        // other — two body lines plus the padding around them.
        //
        // Capped at a third of the band, because on a short window the caption
        // was out-eating the picture it captions: at 1100x700 the whole band is
        // ~105pt, and a fixed 63pt caption left the stage barely taller than
        // its own label. Under the cap the card keeps the name and drops the
        // line under it — `card` checks the room it was actually given rather
        // than clipping. Same defect the thumbnail mode exists to fix, chrome
        // starving the card, one level up.
        let caption_h = (fluid.fs_body() + fluid.fs_body_sm() + cl::space::S6).min(room / 3.0);
        let max_card_h = room.max(caption_h + arrow_w);
        let card_w = card_w.min((max_card_h - caption_h).max(arrow_w) * 16.0 / 9.0);
        let stage_h = card_w * 9.0 / 16.0;
        let card_h = stage_h + caption_h;
        let page_step = (card_w + gap) * across;

        // How many cards the lane hands out. Off, exactly the shelf. On, it
        // cycles — pages grow as the sweep reaches the end, so an untouched
        // shelf costs nothing.
        let shown = if self.endless {
            items.len() + self.pages * PAGE
        } else {
            items.len()
        };

        // Claim the full band up front so whatever follows the carousel lands
        // BELOW it. Painting straight into an allocated rect (rather than
        // letting a nested ScrollArea negotiate its own size) is what keeps the
        // lane inside the hub's content panel instead of bleeding into the
        // Inspector.
        let (band, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), card_h),
            egui::Sense::hover(),
        );

        let left = egui::Rect::from_min_size(band.min, egui::vec2(arrow_w, card_h));
        let lane = egui::Rect::from_min_size(
            band.min + egui::vec2(arrow_w + gap, 0.0),
            egui::vec2(lane_w, card_h),
        );
        let right = egui::Rect::from_min_size(
            egui::pos2(lane.max.x + gap, band.min.y),
            egui::vec2(arrow_w, card_h),
        );

        let at_start = self.scroll_at <= 1.0;
        let mut go_back = false;
        ui.scope_builder(egui::UiBuilder::new().max_rect(left), |ui| {
            go_back = self.arrow(ui, "‹", arrow_w, card_h, !at_start);
        });

        let mut content_w = 0.0_f32;
        ui.scope_builder(egui::UiBuilder::new().max_rect(lane), |ui| {
            // Hard clip: the lane never draws outside its own band.
            ui.set_clip_rect(lane);
            let mut area = egui::ScrollArea::horizontal()
                .id_salt(("author-carousel", self.shelf.slug()))
                .max_width(lane_w)
                .max_height(card_h)
                .auto_shrink([false, false]);
            if let Some(x) = self.scroll_to.take() {
                area = area.horizontal_scroll_offset(x);
            }
            let out = area.show(ui, |ui| {
                // Clip HERE — on the ui that actually paints the cards. Setting
                // it on the parent is not enough: the scroll area builds its own
                // child ui, and a card painted through that one would otherwise
                // run straight out of the lane and over the Inspector.
                ui.set_clip_rect(lane);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for i in 0..shown {
                        let item = &items[i % items.len()];
                        if self.card(ui, item, card_w, stage_h, caption_h, i) {
                            picked = Some((item.id.clone(), Some(item.label.clone())));
                        }
                    }
                });
            });
            self.scroll_at = out.state.offset.x;
            content_w = out.content_size.x;
        });

        // Off, the arrow dims at the true end. On, there is always more.
        let at_end = !self.endless && self.scroll_at + lane_w >= content_w - 1.0;
        let mut go_on = false;
        ui.scope_builder(egui::UiBuilder::new().max_rect(right), |ui| {
            go_on = self.arrow(ui, "›", arrow_w, card_h, !at_end);
        });

        if go_back {
            self.scroll_to = Some((self.scroll_at - page_step).max(0.0));
        }
        if go_on {
            self.scroll_to = Some(self.scroll_at + page_step);
            if self.endless {
                self.pages += 1;
            }
        }

        picked
    }

    /// One sweep arrow. Returns true when tapped; dims when there is no
    /// further to go, rather than vanishing (the target must not move).
    fn arrow(&self, ui: &mut egui::Ui, glyph: &str, w: f32, h: f32, live: bool) -> bool {
        // Full height to hit, but only the middle is painted — a tall bright
        // slab either side of the lane would out-shout the cards it serves.
        let (hit, resp) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::click());
        let face = h * 0.34;
        let rect = egui::Rect::from_center_size(hit.center(), egui::vec2(w, face));
        let style = cl::compose(
            super::SPEC.tone,
            "claymorphic",
            &[],
            if live && resp.hovered() {
                cl::TraitState::Hover
            } else {
                cl::TraitState::Idle
            },
        );
        let painter = ui.painter();
        for sh in &style.shadows {
            painter.rect_filled(
                rect.translate(egui::vec2(sh.offset[0] as f32, sh.offset[1] as f32)),
                egui::CornerRadius::same(style.radius),
                sh.color,
            );
        }
        painter.rect_filled(rect, egui::CornerRadius::same(style.radius), style.fill);
        painter.rect_stroke(
            rect,
            egui::CornerRadius::same(style.radius),
            style.edge,
            egui::StrokeKind::Inside,
        );
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            glyph,
            egui::FontId::proportional(cl::Fluid::of(rect).fs_h3()),
            if live { style.ink } else { cl::ink::MUTED },
        );
        let resp = cl::tip(
            resp,
            if glyph == "‹" {
                "Back a few"
            } else {
                "On a few more"
            },
        );
        live && resp.clicked()
    }

    /// One card: the thing itself, drawn live, with its plain name under it.
    fn card(
        &mut self,
        ui: &mut egui::Ui,
        item: &ShelfItem,
        w: f32,
        stage_h: f32,
        caption_h: f32,
        ix: usize,
    ) -> bool {
        let (rect, resp) =
            ui.allocate_exact_size(egui::vec2(w, stage_h + caption_h), egui::Sense::click());
        let style = cl::compose(
            super::SPEC.tone,
            "neubrutalist",
            &[],
            if resp.hovered() {
                cl::TraitState::Hover
            } else {
                cl::TraitState::Idle
            },
        );
        {
            let painter = ui.painter();
            for sh in &style.shadows {
                painter.rect_filled(
                    rect.translate(egui::vec2(sh.offset[0] as f32, sh.offset[1] as f32)),
                    egui::CornerRadius::same(style.radius),
                    sh.color,
                );
            }
            painter.rect_filled(rect, egui::CornerRadius::same(style.radius), style.fill);
            painter.rect_stroke(
                rect,
                egui::CornerRadius::same(style.radius),
                style.edge,
                egui::StrokeKind::Inside,
            );
        }

        // The stage — the thing drawn for real, over the WELL.
        let stage = egui::Rect::from_min_size(
            rect.min + egui::vec2(cl::space::S2, cl::space::S2),
            egui::vec2(w - cl::space::S2 * 2.0, stage_h),
        );
        ui.painter()
            .rect_filled(stage, cl::RADIUS_SM, cl::surface::WELL);
        ui.scope_builder(egui::UiBuilder::new().max_rect(stage.shrink(2.0)), |ui| {
            // INTERSECT, never replace: the lane already clipped us, and
            // overwriting that with the stage would let a card at the edge paint
            // straight out of the carousel and over whatever sits beside it.
            ui.set_clip_rect(ui.clip_rect().intersect(stage.shrink(2.0)));
            self.paint_item(ui, item, ix);
        });

        // The name and its one line, in plain words.
        let text = egui::Rect::from_min_max(
            egui::pos2(rect.min.x + cl::space::S2, stage.max.y + cl::space::S1),
            rect.max - egui::vec2(cl::space::S2, cl::space::S1),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(text), |ui| {
            ui.set_clip_rect(ui.clip_rect().intersect(text));
            let fluid = cl::Fluid::of(ui.max_rect());
            // Stacked, tight, and never wrapping — the name reads first, the
            // line under it explains, and a long one is cut rather than shoving
            // the card taller than its neighbours.
            // The line under the name is dropped, not clipped, when the band
            // is too short for it. Half a sentence sheared off by an edge
            // reads as a bug; a name on its own reads as a name.
            let name_h = fluid.fs_body() * 1.35;
            let line_h = fluid.fs_body_sm() * 1.35;
            let room_for_the_line = text.height() >= name_h + line_h + cl::space::S1;
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = cl::space::S1;
                ui.add(
                    egui::Label::new(
                        RichText::new(&item.label)
                            .size(fluid.fs_body())
                            .strong()
                            .color(cl::ink::STRONG),
                    )
                    .truncate(),
                );
                if room_for_the_line {
                    ui.add(
                        egui::Label::new(
                            RichText::new(&item.blurb)
                                .size(fluid.fs_body_sm())
                                .color(cl::ink::MUTED),
                        )
                        .truncate(),
                    );
                }
            });
        });

        ui.pilot_track(
            &resp,
            NodeMetadata::new(
                PilotId::new("workbench.author.card").child(&PilotKey::new(format!(
                    "{}/{}",
                    self.shelf.slug(),
                    item.id
                ))),
                egui_pilot_ai::protocol::Role::Button,
                &item.label,
            ),
        );
        cl::tip(resp.clone(), item.tip.as_str());
        resp.clicked()
    }

    /// Read every registry ONCE. Templates and seeds carry state (clocks,
    /// interaction) that a per-frame rebuild would reset mid-sweep.
    fn load_once(&mut self) {
        if self.loaded {
            return;
        }
        self.loaded = true;
        // Land on the first kind that has something behind it, so the strip
        // starts labelled with a real container rather than an empty heading.
        // If none do, the bar says so and the strip stays honest about it.
        let root = dpl_engine_capture::content_root();
        let first = ContainerKind::ALL
            .into_iter()
            .find(|k| !containers(*k, &root).is_empty())
            .unwrap_or(ContainerKind::Episode);
        self.choose_kind(first);
        self.demos = dpl_slides::demos();
        self.seeds = dpl_doc::list_seeds()
            .into_iter()
            .filter_map(|s| {
                dpl_doc::load(&s.dir)
                    .ok()
                    .map(|slide| (s.id, dpl_slides::document::DocSlide::new(slide)))
            })
            .collect();
    }

    /// What is on the current shelf — straight off the registries, never a
    /// hand-list (Census law). An id here is a machine id; the card draws the
    /// label.
    fn shelf_items(&self) -> Vec<ShelfItem> {
        match self.shelf {
            Shelf::Starters => self
                .seeds
                .iter()
                .map(|(id, _)| ShelfItem {
                    id: id.clone(),
                    label: pretty(id),
                    blurb: "a whole card, ready to make yours".into(),
                    tip: "Tap to start from this card".into(),
                })
                .collect(),
            Shelf::Layouts => self
                .demos
                .iter()
                .map(|d| {
                    let spec = d.spec();
                    ShelfItem {
                        id: spec.id.clone(),
                        label: spec.name.clone(),
                        blurb: spec.summary.clone(),
                        tip: format!("Tap to start a {} card", spec.name.to_lowercase()),
                    }
                })
                .collect(),
            Shelf::Combos => dpl_slides::blocks::blocks()
                .iter()
                .map(|b| {
                    let spec = b.spec();
                    ShelfItem {
                        id: spec.id.0.to_string(),
                        label: spec.label.to_string(),
                        blurb: spec.blurb.to_string(),
                        tip: format!("Tap to put {} on a card", spec.label.to_lowercase()),
                    }
                })
                .collect(),
            Shelf::Pieces => dpl_slides::elements::elements()
                .iter()
                .map(|e| {
                    let spec = e.spec();
                    ShelfItem {
                        id: spec.id.0.to_string(),
                        label: spec.label.to_string(),
                        blurb: spec.blurb.to_string(),
                        tip: format!("Tap to put a {} on a card", spec.label.to_lowercase()),
                    }
                })
                .collect(),
            Shelf::Themes => cl::TRAITS
                .iter()
                .map(|t| ShelfItem {
                    id: t.id.to_string(),
                    label: t.label.to_string(),
                    blurb: t.blurb.to_string(),
                    tip: format!("Tap to dress a card in {}", t.label.to_lowercase()),
                })
                .collect(),
            // No registry yet — recipes arrive with the interaction tier, and
            // an empty shelf is more honest than an invented one.
            Shelf::Interactions => Vec::new(),
        }
    }

    /// Draw the thing itself into the card's stage — the real paint path every
    /// other surface uses, so what you sweep past is what you get.
    fn paint_item(&mut self, ui: &mut egui::Ui, item: &ShelfItem, ix: usize) {
        let rect = ui.max_rect();
        match self.shelf {
            Shelf::Starters => {
                if let Some((_, doc)) = self.seeds.iter_mut().find(|(id, _)| *id == item.id) {
                    // As a MINIATURE: a card stage is well under the ramp's
                    // floor, so sizing content TO it leaves a headline at its
                    // minimum and crops the card. A thumbnail is a picture of
                    // the whole slide, which is what a shelf of premades is.
                    doc.thumbnail_ui(ui, rect);
                }
            }
            Shelf::Layouts => {
                if let Some(demo) = self.demos.iter_mut().find(|d| d.spec().id == item.id) {
                    demo.ui(ui);
                }
            }
            Shelf::Combos => {
                if let Some(b) = dpl_slides::blocks::blocks()
                    .iter()
                    .find(|b| b.spec().id.0 == item.id)
                {
                    b.paint(ui);
                }
            }
            Shelf::Pieces => {
                if let Some(e) = dpl_slides::elements::elements()
                    .iter()
                    .find(|e| e.spec().id.0 == item.id)
                {
                    e.paint(ui);
                }
            }
            Shelf::Themes => {
                // A theme has nothing of its own to show, so it dresses a plate:
                // base treatments stand alone, stack traits ride the
                // neubrutalist base — the way they are actually used.
                let def = cl::TRAITS.iter().find(|t| t.id == item.id);
                let (base, stack): (&str, &[&str]) = match def {
                    Some(d) if d.role == "base" => (d.id, &[]),
                    Some(d) => ("neubrutalist", std::slice::from_ref(&d.id)),
                    None => ("neubrutalist", &[]),
                };
                // Vary the tone down the shelf so a sweep reads as motion, not
                // a row of photocopies.
                let tone = TONE_CYCLE[ix % TONE_CYCLE.len()];
                let style = cl::compose(tone, base, stack, cl::TraitState::Idle);
                let plate = rect.shrink(cl::space::S3);
                let painter = ui.painter();
                for shape in style.behind_shapes(plate) {
                    painter.add(shape);
                }
                for sh in &style.shadows {
                    painter.rect_filled(
                        plate.translate(egui::vec2(sh.offset[0] as f32, sh.offset[1] as f32)),
                        egui::CornerRadius::same(style.radius),
                        sh.color,
                    );
                }
                painter.rect_filled(plate, egui::CornerRadius::same(style.radius), style.fill);
                painter.rect_stroke(
                    plate,
                    egui::CornerRadius::same(style.radius),
                    style.edge,
                    egui::StrokeKind::Inside,
                );
            }
            Shelf::Interactions => {}
        }
    }

    /// A shelf with nothing on it says why, rather than looking broken.
    fn empty_shelf(&self, ui: &mut egui::Ui, fluid: &cl::Fluid) {
        ui.label(
            RichText::new("Nothing here yet — and that's on purpose.")
                .size(fluid.fs_body_lg())
                .strong()
                .color(cl::ink::STRONG),
        );
        ui.add_space(cl::space::S1);
        ui.label(
            RichText::new(match self.shelf {
                Shelf::Combos => {
                    "Something only shows up here once you have used the same handful of pieces \
                     together on two or more cards. Until that happens this stays empty, rather \
                     than filling up with things nothing actually uses."
                }
                Shelf::Interactions => {
                    "Nothing to tap yet. When you ask for one — reveal, flip, explode — it gets \
                     built properly on its own first, and then it lives here for good."
                }
                _ => "Nothing has been added to this shelf yet.",
            })
            .small()
            .color(cl::ink::MUTED),
        );
    }

    /// Mint a real, schema-valid draft `slide.json` in `content/inbox/`. The
    /// draft carries the intent text as one block so the mutation honestly
    /// reflects what the person typed. Returns the draft's folder id.
    ///
    /// No-clobber: probes `authored-draft-<n>` for the first free folder, so a
    /// second tap never overwrites the first (mirrors the composer's mint law).
    /// Mint the card into the library (`library/slides/<id>/`) and return
    /// its id and folder. A Layout starts from its component's first
    /// example; a Starter is copied as the operator's own slide (it remembers
    /// the seed it came from — seeds are never edited); a blank card is the
    /// title component holding the operator's words, if any.
    fn mint_draft(&mut self, start: &Start) -> Result<(String, std::path::PathBuf), String> {
        let slide = match start {
            Start::Layout(id) => {
                let contract = dpl_slides::components()
                    .into_iter()
                    .find(|contract| contract.id == *id)
                    .ok_or_else(|| format!("no layout {id:?}"))?;
                dpl_doc::slide_from_example(contract, 0)
                    .ok_or_else(|| format!("layout {id:?} has no example"))?
            }
            Start::Starter(id) => self
                .seeds
                .iter()
                .find(|(seed, _)| seed == id)
                .map(|(_, doc)| doc.slide.fork())
                .ok_or_else(|| format!("no starter {id:?}"))?,
            Start::Blank => {
                let mut slide = dpl_doc::Slide::new("title@1");
                let intent = self.intent.trim();
                if !intent.is_empty() {
                    slide.blocks.push(dpl_doc::Block::Text {
                        id: "title".into(),
                        preset: None,
                        tone: None,
                        text: intent.to_string(),
                        font: None,
                        size: Some("h3".into()),
                        ink: None,
                    });
                }
                slide
            }
        };
        let dir = dpl_doc::library_slide_dir(&dpl_engine_capture::content_root(), &slide.id);
        dpl_doc::save(&dir, &slide).map_err(|e| e.to_string())?;
        Ok((slide.id, dir))
    }
}

#[cfg(test)]
pub(crate) mod background_tests {
    use super::*;
    use egui_kittest::kittest::Queryable as _;

    /// A slide folder with a real, schema-valid document in it.
    pub(crate) fn card(name: &str, background: Option<dpl_doc::Background>) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("dpl-bg-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut slide = dpl_doc::Slide::new("title@1");
        slide.background = background;
        dpl_doc::save(&dir, &slide).unwrap();
        dir
    }

    /// A `CardEditor` over a real card folder — the state every tab takes.
    /// Shared, so a test never builds one a different way from the surface.
    pub(super) fn card_editor(dir: &std::path::Path) -> CardEditor {
        CardEditor {
            title: "Test card".into(),
            editor: dpl_editor::Editor::open(dir).unwrap(),
            tab: CardTab::default(),
            dir: dir.to_path_buf(),
            quote: None,
            pictures: crate::pictures::PictureKind::default(),
            shelf: None,
            preview: dpl_slides::motion::Clock::default(),
        }
    }

    /// Run the tab and hand back the document it left behind.
    ///
    /// A colour name appears in row one AND row two once a first colour is
    /// picked — the same ten colours are offered to blend with. `nth` says
    /// which row's tile to tap: 0 is "make this the colour", 1 is "blend this
    /// into it".
    fn drive_nth(dir: &std::path::Path, label: &str, nth: usize) -> dpl_doc::Slide {
        let mut harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(1280.0, 800.0))
            .build_ui_state(
                |ui, ed: &mut CardEditor| background_tab(ui, ed),
                card_editor(dir),
            );
        harness.run();
        let hits = harness.get_all_by_label(label).collect::<Vec<_>>();
        assert!(hits.len() > nth, "{label:?} has {} tiles", hits.len());
        hits[nth].click();
        harness.run();
        dpl_doc::load(dir).unwrap()
    }

    /// Run the tab for a few frames and hand back the document it left behind.
    fn drive(dir: &std::path::Path, clicks: &[&str]) -> dpl_doc::Slide {
        let mut harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(1280.0, 800.0))
            .build_ui_state(
                |ui, ed: &mut CardEditor| background_tab(ui, ed),
                card_editor(dir),
            );
        harness.run();
        for label in clicks {
            harness.get_by_label(label).click();
            harness.run();
        }
        dpl_doc::load(dir).unwrap()
    }

    /// The gallery gate: every authored colour paints, and tapping one lands
    /// in the card's own document. A picker that draws but does not write is
    /// the exact failure this surface exists to avoid.
    #[test]
    fn tapping_a_colour_writes_it_to_the_card() {
        let dir = card("pick", None);
        let after = drive(&dir, &["Deep lagoon"]);
        std::fs::remove_dir_all(&dir).ok();
        assert_eq!(
            after.background,
            Some(dpl_doc::Background::Mood {
                mood: "deep-lagoon".into()
            })
        );
    }

    /// Row two only exists once row one is answered — and when it is, a
    /// second colour promotes the card to a blend rather than replacing it.
    #[test]
    fn a_second_colour_blends_rather_than_replaces() {
        let dir = card(
            "blend",
            Some(dpl_doc::Background::Mood {
                mood: "ember-drift".into(),
            }),
        );
        // Row two: the tile that blends, not the one that replaces.
        let after = drive_nth(&dir, "Jade current", 1);
        std::fs::remove_dir_all(&dir).ok();
        match after.background {
            Some(dpl_doc::Background::MoodBlend { moods, .. }) => {
                assert_eq!(
                    moods,
                    vec!["ember-drift".to_string(), "jade-current".into()]
                );
            }
            other => panic!("expected a blend, got {other:?}"),
        }
    }

    /// Row three: something drifting over the colour. It rides on a blend, so
    /// a single colour is promoted rather than losing the movement — and the
    /// ref written must be one `document.rs` actually resolves.
    #[test]
    fn movement_over_a_single_colour_promotes_it_to_a_blend() {
        let dir = card(
            "ambience",
            Some(dpl_doc::Background::Mood {
                mood: "violet-static".into(),
            }),
        );
        let after = drive(&dir, &["Rings"]);
        std::fs::remove_dir_all(&dir).ok();
        match after.background {
            Some(dpl_doc::Background::MoodBlend {
                moods, ambience, ..
            }) => {
                assert_eq!(ambience.as_deref(), Some("rings"));
                assert!(moods.iter().all(|m| m == "violet-static"));
            }
            other => panic!("expected a blend carrying the movement, got {other:?}"),
        }
    }

    /// Whatever the tab writes has to survive the validator — an option that
    /// produces a document the app refuses to load is not an option.
    #[test]
    fn every_colour_writes_a_document_that_validates() {
        for colour in crate::backgrounds::colours() {
            let dir = card(&format!("valid-{}", colour.slug), None);
            let after = drive(&dir, &[colour.label.as_str()]);
            let issues = dpl_doc::validate(&after);
            std::fs::remove_dir_all(&dir).ok();
            assert!(issues.is_empty(), "{} → {issues:?}", colour.slug);
        }
    }
}

#[cfg(test)]
mod movement_tests {
    use super::background_tests::*;
    use super::*;
    use egui_kittest::kittest::Queryable as _;

    fn drive_movement(dir: &std::path::Path, label: &str) -> dpl_doc::Slide {
        let mut harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(1280.0, 800.0))
            .build_ui_state(
                |ui, ed: &mut CardEditor| movement_tab(ui, ed),
                card_editor(dir),
            );
        harness.run();
        harness.get_by_label_contains(label).click();
        harness.run();
        dpl_doc::load(dir).unwrap()
    }

    /// The shelf IS the schema's vocabulary plus "follow the show". This
    /// fails the day `ENTERS` grows — which is the point: a new entrance
    /// must reach the picker, not sit in the schema unoffered.
    #[test]
    fn the_movement_row_is_the_schemas_vocabulary() {
        assert_eq!(dpl_doc::enters(), ["rise", "none"]);
        for slug in dpl_doc::enters() {
            assert_ne!(
                movement_label(slug),
                "A new way in",
                "{slug} is allowed by the schema but has no operator word"
            );
        }
    }

    #[test]
    fn picking_a_movement_writes_it_and_still_validates() {
        for (label, expected) in [("Rise into place", "rise"), ("Stay still", "none")] {
            let dir = card(&format!("move-{expected}"), None);
            let after = drive_movement(&dir, label);
            let issues = dpl_doc::validate(&after);
            std::fs::remove_dir_all(&dir).ok();
            assert_eq!(after.enter.as_deref(), Some(expected));
            assert!(issues.is_empty(), "{expected} → {issues:?}");
        }
    }

    /// "Follow the show" clears the override rather than writing a value —
    /// inheriting is an absence in the document, and must stay one.
    #[test]
    fn following_the_show_clears_the_override() {
        let dir = card("move-follow", None);
        // Start from a deliberate override so clearing it is observable.
        {
            let mut ed = dpl_editor::Editor::open(&dir).unwrap();
            ed.set_enter(Some("none".into()));
        }
        let after = drive_movement(&dir, "Follow the show");
        std::fs::remove_dir_all(&dir).ok();
        assert_eq!(after.enter, None, "inheriting must not write a value");
    }
}

#[cfg(test)]
mod overlay_tests {
    use super::background_tests::{card, card_editor};
    use super::*;
    use egui_kittest::kittest::Queryable as _;

    /// The gallery gate for tabs: every registered tab is reachable from the
    /// rail, in BOTH orientations. A tab that exists but cannot be got to is
    /// the same defect as one that never paints.
    #[test]
    fn every_tab_is_reachable_from_the_rail_in_both_orientations() {
        for vertical in [true, false] {
            let dir = card(&format!("rail-{vertical}"), None);
            let ed = card_editor(&dir);
            let mut harness = egui_kittest::Harness::builder()
                .with_size(egui::vec2(1280.0, 800.0))
                .build_ui_state(
                    move |ui, ed: &mut CardEditor| {
                        if vertical {
                            tab_rail(ui, ed, Tone::Amber);
                        } else {
                            tab_rail_wrapped(ui, ed, Tone::Amber);
                        }
                    },
                    ed,
                );
            harness.run();
            for tab in CardTab::ALL {
                harness.get_by_label(tab.label()).click();
                harness.run();
                assert_eq!(
                    harness.state().tab,
                    tab,
                    "{} unreachable with vertical={vertical}",
                    tab.slug()
                );
            }
            std::fs::remove_dir_all(&dir).ok();
        }
    }

    /// Words owns its card because the card is its control; every other tab
    /// gets the preview pane beside it.
    #[test]
    fn only_the_words_tab_owns_its_card() {
        assert!(!CardTab::Words.shows_live_card());
        for tab in CardTab::ALL.into_iter().filter(|t| *t != CardTab::Words) {
            assert!(tab.shows_live_card(), "{} wants a preview", tab.slug());
        }
    }

    /// The live pane draws the card from the editor's own slide, so a change
    /// is on screen before any file round-trip.
    ///
    /// `run_steps`, not `run`: the card is a REAL document render and the
    /// mood blend never rests, so the pane requests a repaint every frame.
    /// A converging harness here would mean the preview had stopped being
    /// live — the thing this pass exists to add.
    #[test]
    fn the_live_pane_shows_a_moving_card_and_says_so() {
        let dir = card("live", None);
        let ed = card_editor(&dir);
        let mut harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(400.0, 400.0))
            .build_ui_state(|ui, ed: &mut CardEditor| live_pane(ui, ed), ed);
        harness.run_steps(4);
        harness.get_by_label_contains("as it looks right now");
        std::fs::remove_dir_all(&dir).ok();
    }
}

#[cfg(test)]
mod quote_tests {
    use super::background_tests::card;

    fn text_ids(slide: &dpl_doc::Slide) -> Vec<String> {
        slide.blocks.iter().map(|b| b.id().to_string()).collect()
    }

    /// Filling all three lines writes exactly the reserved ids, declares the
    /// card a quote, and leaves a document the app will load.
    #[test]
    fn a_full_quote_writes_the_reserved_ids_and_validates() {
        let dir = card("quote-full", None);
        let mut ed = dpl_editor::Editor::open(&dir).unwrap();
        ed.set_quote([
            "The stack is invisible.".into(),
            "The operator".into(),
            "2026-07-29".into(),
        ]);
        let after = dpl_doc::load(&dir).unwrap();
        std::fs::remove_dir_all(&dir).ok();

        assert_eq!(text_ids(&after), dpl_editor::QUOTE_SLOTS);
        assert_eq!(after.component, "quote@1");
        assert!(matches!(
            after.block("quote"),
            Some(dpl_doc::Block::Quote { .. })
        ));
        assert!(
            dpl_doc::validate(&after).is_empty(),
            "{:?}",
            dpl_doc::validate(&after)
        );
    }

    /// A blank line REMOVES its block. An empty text block is a rendering
    /// artefact, not an empty field.
    #[test]
    fn a_blank_line_removes_its_block() {
        let dir = card("quote-blank", None);
        let mut ed = dpl_editor::Editor::open(&dir).unwrap();
        ed.set_quote(["Words".into(), "Someone".into(), "Somewhere".into()]);
        ed.set_quote(["Words".into(), String::new(), "Somewhere".into()]);
        let after = dpl_doc::load(&dir).unwrap();
        std::fs::remove_dir_all(&dir).ok();

        assert_eq!(text_ids(&after), ["quote", "role"], "name should be gone");
        assert!(dpl_doc::validate(&after).is_empty());
    }

    /// Writing twice UPDATES the block, never appends a second with the same
    /// id — the §6 risk this pass named.
    #[test]
    fn writing_twice_updates_rather_than_duplicates() {
        let dir = card("quote-twice", None);
        let mut ed = dpl_editor::Editor::open(&dir).unwrap();
        ed.set_quote(["First".into(), String::new(), String::new()]);
        ed.set_quote(["Second".into(), String::new(), String::new()]);
        let after = dpl_doc::load(&dir).unwrap();
        std::fs::remove_dir_all(&dir).ok();

        assert_eq!(text_ids(&after), ["quote"]);
        match &after.blocks[0] {
            dpl_doc::Block::Quote { text, .. } => assert_eq!(text, "Second"),
            other => panic!("expected the quote block, got {other:?}"),
        }
    }

    /// The card's own lines survive. Adding a quote to a title card adds to
    /// it; it does not restructure it.
    #[test]
    fn a_cards_existing_lines_are_left_alone() {
        let dir = card("quote-alongside", None);
        {
            let mut ed = dpl_editor::Editor::open(&dir).unwrap();
            // Give the card a title of its own, the way a seed would.
            ed.set_quote(["placeholder".into(), String::new(), String::new()]);
            let mut slide = dpl_doc::load(&dir).unwrap();
            slide.blocks.insert(
                0,
                dpl_doc::Block::Text {
                    id: "title".into(),
                    preset: None,
                    tone: None,
                    text: "A card that was here first".into(),
                    font: None,
                    size: Some("h1".into()),
                    ink: Some("strong".into()),
                },
            );
            dpl_doc::save(&dir, &slide).unwrap();
        }
        let mut ed = dpl_editor::Editor::open(&dir).unwrap();
        ed.set_quote(["Real words".into(), "Real person".into(), String::new()]);
        let after = dpl_doc::load(&dir).unwrap();
        std::fs::remove_dir_all(&dir).ok();

        assert!(
            text_ids(&after).contains(&"title".to_string()),
            "{:?}",
            text_ids(&after)
        );
        assert!(dpl_doc::validate(&after).is_empty());
    }

    /// Reading back what was written is what the tab's fields do on open.
    #[test]
    fn the_tab_reads_the_cards_own_quote() {
        let dir = card("quote-read", None);
        let mut ed = dpl_editor::Editor::open(&dir).unwrap();
        ed.set_quote(["W".into(), String::new(), "S".into()]);
        let read = dpl_editor::Editor::open(&dir).unwrap().quote();
        std::fs::remove_dir_all(&dir).ok();
        assert_eq!(read, ["W".to_string(), String::new(), "S".to_string()]);
    }
}

#[cfg(test)]
mod picture_tests {
    use super::background_tests::card;

    /// Putting a picture on writes a Media block the validator accepts, with
    /// `show` taken from the index's own kind — never sniffed.
    #[test]
    fn setting_a_picture_writes_a_valid_media_block() {
        let dir = card("pic-set", None);
        let mut ed = dpl_editor::Editor::open(&dir).unwrap();
        ed.set_media(Some((
            "inbox/shot.png".into(),
            "image".into(),
            "cover".into(),
        )));
        let after = dpl_doc::load(&dir).unwrap();
        std::fs::remove_dir_all(&dir).ok();

        match &after.blocks[0] {
            dpl_doc::Block::Media { show, src, fit, .. } => {
                assert_eq!(show, "image");
                assert_eq!(src, "inbox/shot.png");
                // cover is the schema's default — a sparse document does not
                // write it.
                assert_eq!(*fit, None);
            }
            other => panic!("expected a media block, got {other:?}"),
        }
        assert!(dpl_doc::validate(&after).is_empty());
    }

    /// "Show all of it" is the non-default, so it IS written.
    #[test]
    fn contain_is_written_because_it_is_not_the_default() {
        let dir = card("pic-fit", None);
        let mut ed = dpl_editor::Editor::open(&dir).unwrap();
        ed.set_media(Some((
            "inbox/shot.png".into(),
            "image".into(),
            "contain".into(),
        )));
        let after = dpl_doc::load(&dir).unwrap();
        std::fs::remove_dir_all(&dir).ok();
        match &after.blocks[0] {
            dpl_doc::Block::Media { fit, .. } => assert_eq!(fit.as_deref(), Some("contain")),
            other => panic!("expected a media block, got {other:?}"),
        }
    }

    /// One picture per card: choosing a second REPLACES rather than appending,
    /// because two backdrops would fight.
    #[test]
    fn a_second_picture_replaces_the_first() {
        let dir = card("pic-replace", None);
        let mut ed = dpl_editor::Editor::open(&dir).unwrap();
        ed.set_media(Some(("inbox/a.png".into(), "image".into(), "cover".into())));
        ed.set_media(Some(("inbox/b.png".into(), "image".into(), "cover".into())));
        let after = dpl_doc::load(&dir).unwrap();
        std::fs::remove_dir_all(&dir).ok();

        let media: Vec<&dpl_doc::Block> = after
            .blocks
            .iter()
            .filter(|b| matches!(b, dpl_doc::Block::Media { .. }))
            .collect();
        assert_eq!(media.len(), 1, "only one picture may be on a card");
        match media[0] {
            dpl_doc::Block::Media { src, .. } => assert_eq!(src, "inbox/b.png"),
            other => panic!("{other:?}"),
        }
    }

    /// "No picture" removes the block and leaves the card's words alone.
    #[test]
    fn no_picture_clears_it_and_keeps_the_words() {
        let dir = card("pic-clear", None);
        let mut ed = dpl_editor::Editor::open(&dir).unwrap();
        ed.set_quote(["Words".into(), String::new(), String::new()]);
        ed.set_media(Some(("inbox/a.png".into(), "image".into(), "cover".into())));
        ed.set_media(None);
        let after = dpl_doc::load(&dir).unwrap();
        std::fs::remove_dir_all(&dir).ok();

        assert!(
            !after
                .blocks
                .iter()
                .any(|b| matches!(b, dpl_doc::Block::Media { .. })),
            "the picture should be gone"
        );
        assert!(
            after.blocks.iter().any(|b| b.id() == "quote"),
            "the words should remain"
        );
        assert!(dpl_doc::validate(&after).is_empty());
    }

    /// An escaping ref would be rejected by the validator — proving the jail
    /// is real, and therefore that passing index paths through is what keeps
    /// this tab inside it.
    #[test]
    fn an_escaping_ref_would_not_validate() {
        for bad in ["/etc/passwd", "C:\\x.png", "../../escape.png"] {
            let mut slide = dpl_doc::Slide::new("media@1");
            slide.blocks = vec![dpl_doc::Block::media("media", "image", bad)];
            assert!(
                !dpl_doc::validate(&slide).is_empty(),
                "{bad} should be refused by the content jail"
            );
        }
    }
}

#[cfg(test)]
mod place_tests {
    /// A content root with one real episode holding one chapter.
    fn episode_root(name: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("dpl-place-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let ep = root.join("effingai").join("ep04");
        std::fs::create_dir_all(&ep).unwrap();
        dpl_doc::save_episode(
            &ep,
            &dpl_doc::Episode {
                v: dpl_doc::SCHEMA_V,
                id: "ep04".into(),
                title: "Neon Drop".into(),
                chapters: vec![dpl_doc::Chapter {
                    id: "intro".into(),
                    title: "Intro".into(),
                    slides: Vec::new(),
                }],
            },
        )
        .unwrap();
        root
    }

    /// A finished card in the inbox, ready to be filed.
    fn draft(
        root: &std::path::Path,
        name: &str,
        blocks: Vec<dpl_doc::Block>,
    ) -> std::path::PathBuf {
        let dir = root.join("inbox").join(name);
        std::fs::create_dir_all(&dir).unwrap();
        let mut slide = dpl_doc::Slide::new("title@1");
        slide.blocks = blocks;
        dpl_doc::save(&dir, &slide).unwrap();
        dir
    }

    /// The whole point: a card ends up in the episode, the chapter grows, and
    /// the operator's own inbox copy is left exactly where it was.
    #[test]
    fn placing_copies_into_the_chapter_and_leaves_the_draft_alone() {
        let root = episode_root("ok");
        let dir = draft(&root, "authored-draft-1", Vec::new());
        let placed =
            dpl_doc::place_slide(&root, &dir, "effingai", "ep04", "intro", None).expect("place");

        let ep = dpl_doc::load_episode(&root.join("effingai").join("ep04")).unwrap();
        let survived = dir.join("slide.json").is_file();
        std::fs::remove_dir_all(&root).ok();

        assert_eq!(ep.chapters[0].slides, vec![placed.slide]);
        assert!(survived, "the inbox draft must survive — place COPIES");
    }

    /// A refused place must leave the card where it is. An operator who
    /// believes their work is filed when it is not is the worst outcome this
    /// surface can produce.
    #[test]
    fn an_unfinished_card_is_refused_and_survives() {
        let root = episode_root("bad");
        // An unknown size ref is exactly what `validate` exists to catch.
        let dir = draft(
            &root,
            "authored-draft-1",
            vec![dpl_doc::Block::Text {
                id: "title".into(),
                preset: None,
                tone: None,
                text: "hi".into(),
                font: None,
                size: Some("not-a-size".into()),
                ink: None,
            }],
        );
        let outcome = dpl_doc::place_slide(&root, &dir, "effingai", "ep04", "intro", None);
        let ep = dpl_doc::load_episode(&root.join("effingai").join("ep04")).unwrap();
        let survived = dir.join("slide.json").is_file();
        std::fs::remove_dir_all(&root).ok();

        assert!(outcome.is_err(), "an unfinished card must not be placed");
        assert!(
            ep.chapters[0].slides.is_empty(),
            "nothing may reach the chapter"
        );
        assert!(survived, "the draft must still be there to fix");
    }

    /// The count the strip shows is a fact about the folder, so it has to come
    /// FROM the folder after a place — never an increment.
    #[test]
    fn the_count_after_placing_is_what_is_on_disk() {
        let root = episode_root("count");
        for n in 1..=2 {
            let dir = draft(&root, &format!("authored-draft-{n}"), Vec::new());
            dpl_doc::place_slide(&root, &dir, "effingai", "ep04", "intro", None).expect("place");
        }
        let found = crate::containers::containers(crate::containers::ContainerKind::Episode, &root);
        std::fs::remove_dir_all(&root).ok();

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].count, 2, "the census must read the placed cards");
        assert_eq!(found[0].chapters.len(), 1);
    }

    /// The wiring itself: the surface's own gesture files the card, drops it
    /// from the sitting, and re-reads the count from disk.
    ///
    /// Provable only because `place_draft` takes the root — `content_root()`
    /// is a process-wide `OnceLock` and cannot be pointed at a scratch dir
    /// from a test.
    #[test]
    fn the_surfaces_own_gesture_files_the_card_and_updates_itself() {
        let root = episode_root("surface");
        let dir = draft(&root, "authored-draft-1", Vec::new());

        let mut surface = super::AuthorSurface {
            offered: crate::containers::containers(
                crate::containers::ContainerKind::Episode,
                &root,
            ),
            into: Some(0),
            kind: Some(crate::containers::ContainerKind::Episode),
            ..Default::default()
        };
        surface.drafts.push(super::Draft {
            slide: "authored-draft-1".into(),
            label: "New card".into(),
            dir,
            doc: None,
        });

        surface.place_draft(0, "intro", &root);
        let ep = dpl_doc::load_episode(&root.join("effingai").join("ep04")).unwrap();
        let drafts_left = surface.drafts.len();
        let count = surface.offered[0].count;
        let status = surface.status.clone();
        std::fs::remove_dir_all(&root).ok();

        assert_eq!(
            ep.chapters[0].slides.len(),
            1,
            "the card must reach the chapter"
        );
        assert_eq!(drafts_left, 0, "a filed card leaves the sitting's list");
        assert_eq!(count, 1, "the count is re-read from disk, not incremented");
        assert!(status.contains("put"), "the operator is told: {status:?}");
    }

    /// A refused place leaves the card in the strip AND says why — the
    /// failure mode this whole gesture must never have is silent success.
    #[test]
    fn a_refused_place_keeps_the_card_and_explains() {
        let root = episode_root("surface-bad");
        let dir = draft(
            &root,
            "authored-draft-1",
            vec![dpl_doc::Block::Text {
                id: "title".into(),
                preset: None,
                tone: None,
                text: "hi".into(),
                font: None,
                size: Some("not-a-size".into()),
                ink: None,
            }],
        );

        let mut surface = super::AuthorSurface {
            offered: crate::containers::containers(
                crate::containers::ContainerKind::Episode,
                &root,
            ),
            into: Some(0),
            ..Default::default()
        };
        surface.drafts.push(super::Draft {
            slide: "authored-draft-1".into(),
            label: "New card".into(),
            dir,
            doc: None,
        });

        surface.place_draft(0, "intro", &root);
        let drafts_left = surface.drafts.len();
        let status = surface.status.clone();
        std::fs::remove_dir_all(&root).ok();

        assert_eq!(drafts_left, 1, "a refused card must stay in the strip");
        assert!(
            status.contains("could not put that card in"),
            "the reason must reach the operator: {status:?}"
        );
    }

    /// Only an episode can receive a card, and the others say why in words
    /// rather than offering a control that cannot work.
    #[test]
    fn only_an_episode_takes_a_card_and_the_rest_explain() {
        use crate::containers::ContainerKind;
        assert!(ContainerKind::Episode.cannot_take_a_card().is_none());
        for kind in [
            ContainerKind::Guide,
            ContainerKind::Lesson,
            ContainerKind::Deck,
        ] {
            let why = kind.cannot_take_a_card().expect("must explain");
            assert!(why.len() > 40, "{} shrugs: {why:?}", kind.slug());
        }
        // A guide is a different content model, not a shortfall — its wording
        // must not imply it is unfinished.
        let guide = ContainerKind::Guide.cannot_take_a_card().unwrap();
        assert!(
            !guide.contains("yet"),
            "a guide is not waiting on anything: {guide:?}"
        );
    }
}

#[cfg(test)]
mod dashboard_tests {
    use super::*;

    /// Every harness seam must land on the CONTENT surface. The dashboard is
    /// where a person starts; a proof script has already made its choice, and
    /// every committed proof shot depends on this.
    #[test]
    fn every_drill_enters_the_content_screen() {
        let mut s = AuthorSurface::default();
        s.container_drill("episode");
        assert_eq!(s.screen, Screen::Content, "--author-container");

        let mut s = AuthorSurface::default();
        s.card_tab_drill("background");
        assert_eq!(s.screen, Screen::Content, "--card-tab");

        // The default is still the dashboard — that is the whole point.
        assert_eq!(AuthorSurface::default().screen, Screen::Dashboard);
    }

    /// The "to build with" count summarises the registries the Library shows.
    /// A lane that quoted a number nobody could reach would be the padding
    /// this lane keeps refusing.
    #[test]
    fn the_build_with_count_is_the_registries() {
        let mut s = AuthorSurface::default();
        s.load_once();
        let expected = s.demos.len()
            + s.seeds.len()
            + dpl_slides::elements::elements().len()
            + dpl_slides::blocks::blocks().len()
            + cl::TRAITS.len();
        assert_eq!(s.to_build_with(), expected);
        assert!(expected > 0, "the kit is not empty");
    }

    /// "When" is coarse on purpose: the folder's mtime is the only fact, and
    /// a precise timestamp would imply a history this surface does not keep.
    #[test]
    fn when_is_coarse_and_never_a_timestamp() {
        use std::time::{Duration, SystemTime};
        let now = SystemTime::now();
        assert_eq!(coarse_when(Some(now)), "in the last hour");
        assert_eq!(
            coarse_when(Some(now - Duration::from_secs(30 * 3600))),
            "yesterday"
        );
        assert_eq!(
            coarse_when(Some(now - Duration::from_secs(400 * 3600))),
            "a while ago"
        );
        assert_eq!(coarse_when(None), "unknown");
    }
}
