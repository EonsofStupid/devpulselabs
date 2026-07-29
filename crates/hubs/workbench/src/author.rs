//! Authoring surface — say what you want, then sweep and tap.
//!
//! Pass A landed the spine: an intent bar and one honest mutation — a draft
//! card is a REAL, schema-valid `slide.json` in the content inbox, and
//! `BuildEvent::CardAuthored` publishes AFTER the write (emit-with-mutation).
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
//! Vocabulary: the shelves read Starters · Layouts · Combos · Pieces · Looks ·
//! Interactions. What the code files those under lives in [`Shelf::slug`] and
//! appears nowhere on screen (the UI law, DS3).
//!
//! Law: the surface PUBLISHES `build.*` and never SUBSCRIBES to it
//! (anti-loop) — the coachable-signal catalog forwards the fact to touchpoints
//! so AI/coach overlays react without the surface self-triggering.

use dpl_coldlight::{self as cl, Tone};
use dpl_slides::SlideTemplate;
use egui::RichText;
use egui_pilot_ai::{NodeMetadata, PilotId, PilotKey, UiPilotExt};

/// How many extra cards one sweep adds when the shelf is set to keep going.
const PAGE: usize = 6;

/// Tones a look-plate cycles through down the shelf, so a sweep reads as
/// motion rather than a row of photocopies.
const TONE_CYCLE: [Tone; 4] = [Tone::Cyan, Tone::Amber, Tone::Magenta, Tone::Coral];

/// A folder id as a person would say it: `title-card` → "Title card".
fn pretty(id: &str) -> String {
    let mut out = id.replace(['-', '_'], " ");
    if let Some(first) = out.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    out
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
    Looks,
    Interactions,
}

impl Shelf {
    pub(crate) const ALL: [Shelf; 6] = [
        Shelf::Starters,
        Shelf::Layouts,
        Shelf::Combos,
        Shelf::Pieces,
        Shelf::Looks,
        Shelf::Interactions,
    ];

    /// Stable machine id — never drawn.
    pub(crate) fn slug(self) -> &'static str {
        match self {
            Shelf::Starters => "seeds",
            Shelf::Layouts => "templates",
            Shelf::Combos => "blocks",
            Shelf::Pieces => "elements",
            Shelf::Looks => "traits",
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
            Shelf::Looks => "Looks",
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
            Shelf::Looks => "how it's dressed",
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
            Shelf::Looks => "How a card is dressed — pick one, stack more on top",
            Shelf::Interactions => "What happens when someone taps the card",
        }
    }
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
    /// A hint for the next draft index; `mint_draft` still probes disk for a
    /// free folder so nothing is ever clobbered.
    next_ix: u32,
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
}

impl AuthorSurface {
    pub(crate) fn ui(&mut self, ui: &mut egui::Ui, fabric: &Option<dpl_fabric::Fabric>) {
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
        self.shelf_picker(ui);
        let picked = self.carousel(ui);

        ui.add_space(cl::space::S3);
        let seed = cl::tip(
            ui.label(
                RichText::new("+ Start a blank card")
                    .strong()
                    .color(Tone::Amber.color()),
            )
            .interact(egui::Sense::click()),
            "Make a card with nothing on it but your words, and add to it from there.",
        );

        // ONE authoring path: a tapped card and the blank button both land here,
        // so a card minted by a tap is the same real document as any other.
        let request = picked.or_else(|| seed.clicked().then(|| ("blank".to_string(), None)));
        if let Some((from, label)) = request {
            match self.mint_draft() {
                Ok(slide) => {
                    self.status = match &label {
                        Some(l) => format!("started “{l}” — saved as inbox/{slide}"),
                        None => format!("started a blank card — saved as inbox/{slide}"),
                    };
                    // Emit-with-mutation: the fact is published AFTER the write
                    // succeeds, never before. show/episode are empty until the
                    // card is placed into an episode (Studio's `SlidePlaced`).
                    if let Some(f) = fabric {
                        f.publish_build(dpl_fabric::BuildEvent::CardAuthored {
                            show: String::new(),
                            episode: String::new(),
                            slide,
                        });
                    }
                }
                Err(e) => {
                    self.status = format!("could not start a card: {e} (nothing was written)")
                }
            }
            let _ = from;
        }

        if !self.status.is_empty() {
            ui.add_space(cl::space::S2);
            ui.label(RichText::new(&self.status).small().color(cl::ink::BASE));
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
                    ui.label(
                        RichText::new(shelf.label())
                            .size(cl::Fluid::of(ui.max_rect()).fs_body_lg())
                            .strong()
                            .color(if active {
                                super::SPEC.tone.color()
                            } else {
                                cl::ink::MUTED
                            }),
                    )
                    .interact(egui::Sense::click()),
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

    /// ONE carousel: big cards, swept sideways. Returns what was tapped.
    ///
    /// Three cards across whatever the window, so the targets stay thumb-sized
    /// on the 75" prop and the layout never needs a magic width.
    fn carousel(&mut self, ui: &mut egui::Ui) -> Option<(String, Option<String>)> {
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
        // Room under the stage for a name AND its line, not one on top of the
        // other — two body lines plus the padding around them.
        let caption_h = fluid.fs_body() + fluid.fs_body_sm() + cl::space::S6;
        // Width proposes, HEIGHT disposes. A tall card on a short window pushed
        // its own captions — and everything under the carousel — off the bottom
        // edge. Whichever axis is tighter decides the card, so the band always
        // fits the room it was given.
        let room_below = fluid.fs_body_lg() + cl::space::S6;
        let max_card_h = (ui.available_height() - room_below).max(caption_h + arrow_w);
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
                ui.add(
                    egui::Label::new(
                        RichText::new(&item.blurb)
                            .size(fluid.fs_body_sm())
                            .color(cl::ink::MUTED),
                    )
                    .truncate(),
                );
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
                        id: spec.id.0.to_string(),
                        label: spec.label.to_string(),
                        blurb: spec.blurb.to_string(),
                        tip: format!("Tap to start a {} card", spec.label.to_lowercase()),
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
            Shelf::Looks => cl::TRAITS
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
                    doc.ui(ui, rect);
                }
            }
            Shelf::Layouts => {
                if let Some(demo) = self.demos.iter_mut().find(|d| d.spec().id.0 == item.id) {
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
            Shelf::Looks => {
                // A look has nothing of its own to show, so it dresses a plate:
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
    fn mint_draft(&mut self) -> Result<String, String> {
        let inbox = dpl_engine_capture::content_root().join("inbox");

        let mut n = self.next_ix.max(1);
        let (id, dir) = loop {
            let id = format!("authored-draft-{n}");
            let dir = inbox.join(&id);
            if !dir.exists() {
                break (id, dir);
            }
            n += 1;
            if n > 100_000 {
                return Err("inbox is full of drafts".into());
            }
        };
        self.next_ix = n + 1;

        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

        let intent = self.intent.trim();
        let blocks = if intent.is_empty() {
            Vec::new()
        } else {
            vec![dpl_doc::Block::Text {
                id: "intent".into(),
                text: intent.to_string(),
                font: None,
                size: "h3".into(),
                ink: None,
            }]
        };
        let slide = dpl_doc::Slide {
            v: dpl_doc::SCHEMA_V,
            template: "title".into(),
            background: None,
            blocks,
            enter: None,
            reveals: Vec::new(),
        };
        dpl_doc::save(&dir, &slide).map_err(|e| e.to_string())?;
        Ok(id)
    }
}
