//! The palette's paint layer — tabs, chips, the REC pill, and the C2
//! forced-metadata Save card. Immediate viewports PAINT; every mutation
//! rides `PaletteActions` back to the owner in `lib.rs` (the Save card's
//! text fields are the one exception: they edit the `PendingGuide` the
//! owner lends for the frame).

use dpl_coldlight::{self as cl, Tone};
use egui::{Align2, Color32, CornerRadius, FontId, Rect, RichText, Sense, StrokeKind};
use egui_pilot_ai::protocol::Role as PilotRole;
use egui_pilot_ai::{NodeMetadata, PilotId, UiPilotExt};

use crate::{PendingGuide, TONE};

const PALETTE_TITLE: PilotId = PilotId::new("viziwiz.title");
const PALETTE_CLOSE: PilotId = PilotId::new("viziwiz.close");
const TAB_IMG: PilotId = PilotId::new("viziwiz.tab.img");
const TAB_VIDEO: PilotId = PilotId::new("viziwiz.tab.video");
const TAB_SESSION: PilotId = PilotId::new("viziwiz.tab.session");
const TAB_STEPS: PilotId = PilotId::new("viziwiz.tab.steps");
const REGION_OPEN: PilotId = PilotId::new("viziwiz.region.open");
const SNAP_FULL: PilotId = PilotId::new("viziwiz.snap.full");
const FREEFORM: PilotId = PilotId::new("viziwiz.freeform");
const SNAP_WINDOW: PilotId = PilotId::new("viziwiz.snap.window");
const REFRESH_WINDOWS: PilotId = PilotId::new("viziwiz.windows.refresh");
const VIDEO_START: PilotId = PilotId::new("viziwiz.video.start");
const VIDEO_STOP: PilotId = PilotId::new("viziwiz.video.stop");
const VIDEO_TARGET: PilotId = PilotId::new("viziwiz.video.target");
const VIDEO_FORMAT: PilotId = PilotId::new("viziwiz.video.format");
const VIDEO_AUDIO: PilotId = PilotId::new("viziwiz.video.audio");
const VIDEO_CAP: PilotId = PilotId::new("viziwiz.video.cap");
const SESSION_START: PilotId = PilotId::new("viziwiz.session.start");
const SESSION_STOP: PilotId = PilotId::new("viziwiz.session.stop");
const STEPS_START: PilotId = PilotId::new("viziwiz.steps.start");
const STEPS_MARK: PilotId = PilotId::new("viziwiz.steps.mark");
const STEPS_STOP: PilotId = PilotId::new("viziwiz.steps.stop");
const DELAY_CYCLE: PilotId = PilotId::new("viziwiz.delay");
const RESNIP: PilotId = PilotId::new("viziwiz.resnip");
const GUIDE_NAME: PilotId = PilotId::new("viziwiz.guide.name");
const GUIDE_TAGS: PilotId = PilotId::new("viziwiz.guide.tags");
const GUIDE_DEST: PilotId = PilotId::new("viziwiz.guide.dest");
const GUIDE_SAVE: PilotId = PilotId::new("viziwiz.guide.save");
const GUIDE_DISCARD: PilotId = PilotId::new("viziwiz.guide.discard");

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tab {
    Img,
    Video,
    Session,
    Steps,
}

impl Tab {
    const ALL: [Tab; 4] = [Tab::Img, Tab::Video, Tab::Session, Tab::Steps];
    fn icon(self) -> &'static str {
        match self {
            Tab::Img => cl::icon::IMG,
            Tab::Video => cl::icon::VIDEO,
            Tab::Session => cl::icon::SESSION,
            Tab::Steps => cl::icon::STEPS,
        }
    }
    fn label(self) -> &'static str {
        match self {
            Tab::Img => "Img",
            Tab::Video => "Video",
            Tab::Session => "Session",
            Tab::Steps => "Steps",
        }
    }

    fn pilot_id(self) -> PilotId {
        match self {
            Self::Img => TAB_IMG,
            Self::Video => TAB_VIDEO,
            Self::Session => TAB_SESSION,
            Self::Steps => TAB_STEPS,
        }
    }
}

/// What the palette closure asks the owner to do (immediate viewports
/// paint; the owner mutates).
#[derive(Default)]
pub(crate) struct PaletteActions {
    pub(crate) tab: Option<Tab>,
    pub(crate) close: bool,
    pub(crate) snap_full: bool,
    pub(crate) snap_window: bool,
    pub(crate) begin_region: bool,
    pub(crate) begin_free: bool,
    pub(crate) resnip: bool,
    pub(crate) cycle_delay: bool,
    pub(crate) refresh_windows: bool,
    pub(crate) pick_window: Option<usize>,
    pub(crate) video_start: bool,
    pub(crate) video_stop: bool,
    pub(crate) cycle_format: bool,
    pub(crate) cycle_audio: bool,
    pub(crate) cycle_cap: bool,
    pub(crate) toggle_video_target: bool,
    pub(crate) pick_audio: Option<usize>,
    pub(crate) refresh_audio: bool,
    pub(crate) session_start: bool,
    pub(crate) session_stop: bool,
    pub(crate) steps_start: bool,
    pub(crate) steps_mark: bool,
    pub(crate) steps_stop: bool,
    pub(crate) guide_save: bool,
    pub(crate) guide_discard: bool,
}

/// Everything the palette closure READS this frame — bundled so the
/// paint fn stays inside the argument budget without an `#[allow]`
/// (the workspace law).
pub(crate) struct PaletteView<'a> {
    pub(crate) tab: Tab,
    pub(crate) status: &'a str,
    pub(crate) windows: &'a [String],
    pub(crate) picked: usize,
    pub(crate) video_elapsed: Option<u64>,
    pub(crate) video_window: bool,
    pub(crate) format_label: &'static str,
    pub(crate) audio_label: &'static str,
    pub(crate) audio_silent: bool,
    pub(crate) cap_label: &'static str,
    pub(crate) audio_devices: &'a [String],
    pub(crate) picked_audio: usize,
    pub(crate) session_state: Option<(u64, usize)>,
    pub(crate) steps_note: &'a str,
    pub(crate) delay: u8,
}

pub(crate) fn palette(
    ui: &mut egui::Ui,
    view: &PaletteView<'_>,
    pending: Option<&mut PendingGuide>,
    actions: &mut PaletteActions,
) {
    let PaletteView {
        tab,
        status,
        windows,
        picked,
        video_elapsed,
        video_window,
        format_label,
        audio_label,
        audio_silent,
        cap_label,
        audio_devices,
        picked_audio,
        session_state,
        steps_note,
        delay,
    } = *view;
    // Title row: identity + close.
    ui.horizontal(|ui| {
        ui.label(RichText::new(cl::icon::SNIP).color(TONE.color()).size(14.0));
        let title = ui.label(RichText::new("ViziWiz").strong().color(cl::ink::STRONG));
        ui.pilot_track(
            &title,
            NodeMetadata::new(PALETTE_TITLE, PilotRole::Heading, "ViziWiz"),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if pilot_chip(
                ui,
                PALETTE_CLOSE,
                cl::icon::CLOSE,
                "",
                TONE,
                "Close the palette",
            ) {
                actions.close = true;
            }
        });
    });

    // The tab strip.
    ui.horizontal(|ui| {
        for t in Tab::ALL {
            let active = t == tab;
            let text = format!("{} {}", t.icon(), t.label());
            let rich = if active {
                RichText::new(text).color(TONE.color())
            } else {
                RichText::new(text).color(cl::ink::MUTED)
            };
            let tab_tip = match t {
                Tab::Img => "Snap still pictures of your screen",
                Tab::Video => "Record your screen as video",
                Tab::Session => "Record the screen and the app's own events together",
                Tab::Steps => "Turn your clicks into a step-by-step guide",
            };
            let response = cl::tip(ui.selectable_label(active, rich), tab_tip);
            let mut metadata =
                NodeMetadata::new(t.pilot_id(), PilotRole::Tab, t.label().to_string());
            metadata.selected = Some(active);
            ui.pilot_track(&response, metadata);
            if response.clicked() {
                actions.tab = Some(t);
            }
        }
    });
    ui.separator();

    match tab {
        Tab::Img => {
            ui.horizontal(|ui| {
                if pilot_chip(
                    ui,
                    REGION_OPEN,
                    cl::icon::REGION,
                    "Region",
                    TONE,
                    "Draw a box around the part of the screen you want",
                ) {
                    actions.begin_region = true;
                }
                if pilot_chip(
                    ui,
                    SNAP_FULL,
                    cl::icon::FULL,
                    "Full",
                    TONE,
                    "Snap the whole screen",
                ) {
                    actions.snap_full = true;
                }
                if pilot_chip(
                    ui,
                    FREEFORM,
                    cl::icon::FREE,
                    "Free",
                    TONE,
                    "Draw any shape around what you want",
                ) {
                    actions.begin_free = true;
                }
                // The delay chip cycles 0 → 3 → 5 → 10 seconds (C1).
                let delay_label = if delay == 0 {
                    "0s".to_string()
                } else {
                    format!("{delay}s")
                };
                if pilot_chip(
                    ui,
                    DELAY_CYCLE,
                    "",
                    &delay_label,
                    TONE,
                    "Wait before snapping — taps cycle 0s, 3s, 5s, 10s",
                ) {
                    actions.cycle_delay = true;
                }
                if pilot_chip(
                    ui,
                    RESNIP,
                    "",
                    "Re-snip",
                    TONE,
                    "Snap the same area again, no re-drawing",
                ) {
                    actions.resnip = true;
                }
            });
            ui.add_space(cl::space::S2);
            ui.horizontal(|ui| {
                ui.label(RichText::new(cl::icon::WINDOW).color(cl::ink::MUTED));
                let current = windows
                    .get(picked)
                    .cloned()
                    .unwrap_or_else(|| "no windows scanned".into());
                egui::ComboBox::from_id_salt("viziwiz-window")
                    .width(220.0)
                    .selected_text(RichText::new(truncate(&current, 30)).small())
                    .show_ui(ui, |ui| {
                        for (i, w) in windows.iter().enumerate() {
                            if ui.selectable_label(i == picked, truncate(w, 44)).clicked() {
                                actions.pick_window = Some(i);
                            }
                        }
                    });
                if pilot_chip(
                    ui,
                    SNAP_WINDOW,
                    cl::icon::SNIP,
                    "Snap",
                    TONE,
                    "Snap just the chosen window",
                ) {
                    actions.snap_window = true;
                }
                if cl::tip(
                    ui.pilot_button(REFRESH_WINDOWS, "↻"),
                    "Look for windows again",
                )
                .clicked()
                {
                    actions.refresh_windows = true;
                }
            });
            ui.label(
                RichText::new(
                    "Region arms · arrows nudge · Enter captures. Free: click a polygon closed.",
                )
                .small()
                .color(cl::ink::SUBTLE),
            );
        }
        Tab::Video => match video_elapsed {
            Some(secs) => {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(cl::icon::REC)
                            .color(Tone::Coral.color())
                            .size(15.0),
                    );
                    ui.label(
                        RichText::new(format!("REC {:02}:{:02}", secs / 60, secs % 60))
                            .monospace()
                            .color(Tone::Coral.color()),
                    );
                    if pilot_chip(
                        ui,
                        VIDEO_STOP,
                        cl::icon::STOP,
                        "Stop",
                        TONE,
                        "Stop and save the recording",
                    ) {
                        actions.video_stop = true;
                    }
                });
            }
            None => {
                // The C3 matrix: cycling chips (touch law) + the device
                // list when a mode records audio.
                ui.horizontal(|ui| {
                    let target = if video_window { "Window" } else { "Screen" };
                    if pilot_chip(
                        ui,
                        VIDEO_TARGET,
                        "",
                        target,
                        TONE,
                        "Record the whole screen, or just one window",
                    ) {
                        actions.toggle_video_target = true;
                    }
                    if pilot_chip(
                        ui,
                        VIDEO_FORMAT,
                        "",
                        format_label,
                        TONE,
                        "Video format — MP4 plays everywhere",
                    ) {
                        actions.cycle_format = true;
                    }
                    if pilot_chip(
                        ui,
                        VIDEO_AUDIO,
                        "",
                        audio_label,
                        TONE,
                        "Record silent, computer sound, or your mic",
                    ) {
                        actions.cycle_audio = true;
                    }
                    if pilot_chip(
                        ui,
                        VIDEO_CAP,
                        "",
                        cap_label,
                        TONE,
                        "Stop the recording automatically after this long",
                    ) {
                        actions.cycle_cap = true;
                    }
                });
                if !audio_silent {
                    ui.horizontal(|ui| {
                        let current = audio_devices
                            .get(picked_audio)
                            .cloned()
                            .unwrap_or_else(|| "no audio devices found".into());
                        egui::ComboBox::from_id_salt("viziwiz-audio-device")
                            .width(220.0)
                            .selected_text(RichText::new(truncate(&current, 30)).small())
                            .show_ui(ui, |ui| {
                                for (i, d) in audio_devices.iter().enumerate() {
                                    if ui
                                        .selectable_label(i == picked_audio, truncate(d, 44))
                                        .clicked()
                                    {
                                        actions.pick_audio = Some(i);
                                    }
                                }
                            });
                        if cl::tip(ui.button("↻"), "Look for audio devices again").clicked() {
                            actions.refresh_audio = true;
                        }
                    });
                }
                ui.horizontal(|ui| {
                    if pilot_chip(
                        ui,
                        VIDEO_START,
                        cl::icon::REC,
                        "Record",
                        TONE,
                        "Start recording with the settings above",
                    ) {
                        actions.video_start = true;
                    }
                });
                ui.label(
                    RichText::new(
                        "720p · Stop when done (or the cap stops it) · System audio needs a loopback device.",
                    )
                    .small()
                    .color(cl::ink::SUBTLE),
                );
            }
        },
        Tab::Session => match session_state {
            Some((secs, events)) => {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(cl::icon::REC)
                            .color(Tone::Coral.color())
                            .size(15.0),
                    );
                    ui.label(
                        RichText::new(format!("REC {:02}:{:02}", secs / 60, secs % 60))
                            .monospace()
                            .color(Tone::Coral.color()),
                    );
                    ui.label(
                        RichText::new(format!("{events} events"))
                            .small()
                            .color(cl::ink::SUBTLE),
                    );
                    if pilot_chip(
                        ui,
                        SESSION_STOP,
                        cl::icon::STOP,
                        "Stop",
                        TONE,
                        "Stop and save the session",
                    ) {
                        actions.session_stop = true;
                    }
                });
            }
            None => {
                ui.horizontal(|ui| {
                    if pilot_chip(
                        ui,
                        SESSION_START,
                        cl::icon::REC,
                        "Record session",
                        TONE,
                        "Record the screen AND what the app does, together",
                    ) {
                        actions.session_start = true;
                    }
                });
                ui.label(
                    RichText::new(
                        "Screen video + a typed event track (routes, beats) — one session artifact.",
                    )
                    .small()
                    .color(cl::ink::SUBTLE),
                );
            }
        },
        Tab::Steps => match pending {
            // A stopped guide is PENDING — the forced-metadata Save card
            // (C2). Starting a new recording would drop it, so the card
            // replaces the Start chip until Save or Discard decides.
            Some(p) => save_card(ui, p, actions),
            None => {
                ui.horizontal(|ui| {
                    if pilot_chip(
                        ui,
                        STEPS_START,
                        cl::icon::REC,
                        "Start guide",
                        TONE,
                        "Every click you make becomes a step-by-step picture guide",
                    ) {
                        actions.steps_start = true;
                    }
                });
                ui.label(
                    RichText::new("Every click/Enter auto-snaps a step; Ctrl+Shift+9 toggles, Ctrl+Shift+X marks — from any app.")
                        .small()
                        .color(cl::ink::SUBTLE),
                );
                if !steps_note.is_empty() {
                    ui.label(
                        RichText::new(steps_note)
                            .small()
                            .monospace()
                            .color(cl::ink::MUTED),
                    );
                }
            }
        },
    }

    ui.add_space(cl::space::S2);
    if !status.is_empty() {
        ui.label(
            RichText::new(status)
                .small()
                .monospace()
                .color(cl::ink::BASE),
        );
    }
}

/// The forced-metadata Save card (C2): kebab name (auto `_v1` on save),
/// ≥1 tag, an episode/General destination — the engine's `normalize_meta`
/// refuses anything less and the refusal lands in the status line while
/// the pending guide waits.
fn save_card(ui: &mut egui::Ui, p: &mut PendingGuide, actions: &mut PaletteActions) {
    ui.label(
        RichText::new(format!("Save your guide — {} steps", p.count))
            .strong()
            .color(cl::ink::STRONG),
    );
    ui.add_space(cl::space::S1);
    ui.horizontal(|ui| {
        ui.label(RichText::new("name").small().color(cl::ink::MUTED));
        let name = ui.add(
            egui::TextEdit::singleline(&mut p.name)
                .hint_text("kebab-case")
                .desired_width(200.0),
        );
        ui.pilot_track(
            &name,
            NodeMetadata::new(GUIDE_NAME, PilotRole::TextInput, "guide name").value(p.name.clone()),
        );
    });
    ui.horizontal(|ui| {
        ui.label(RichText::new("tags").small().color(cl::ink::MUTED));
        let tags = ui.add(
            egui::TextEdit::singleline(&mut p.tags)
                .hint_text("space or comma separated")
                .desired_width(200.0),
        );
        ui.pilot_track(
            &tags,
            NodeMetadata::new(GUIDE_TAGS, PilotRole::TextInput, "guide tags").value(p.tags.clone()),
        );
    });
    ui.horizontal(|ui| {
        if ui.selectable_label(p.library, "General").clicked() {
            p.library = true;
        }
        let episode = ui.selectable_label(!p.library, "Episode");
        ui.pilot_track(
            &episode,
            NodeMetadata::new(GUIDE_DEST, PilotRole::Button, "destination").value(if p.library {
                "general"
            } else {
                "episode"
            }),
        );
        if episode.clicked() {
            p.library = false;
        }
        if !p.library {
            ui.add(
                egui::TextEdit::singleline(&mut p.show)
                    .hint_text("show")
                    .desired_width(80.0),
            );
            ui.add(
                egui::TextEdit::singleline(&mut p.episode)
                    .hint_text("episode")
                    .desired_width(80.0),
            );
        }
    });
    ui.add_space(cl::space::S1);
    ui.horizontal(|ui| {
        if pilot_chip(
            ui,
            GUIDE_SAVE,
            "",
            "Save",
            TONE,
            "File the guide with this name and tags",
        ) {
            actions.guide_save = true;
        }
        if pilot_chip(
            ui,
            GUIDE_DISCARD,
            "",
            "Discard",
            TONE,
            "Throw this recording away — the steps are deleted",
        ) {
            actions.guide_discard = true;
        }
    });
}

/// The REC pill — the palette collapsed while Steps records.
pub(crate) fn rec_pill(ui: &mut egui::Ui, count: u32, actions: &mut PaletteActions) {
    ui.horizontal_centered(|ui| {
        // The pulse: alpha breathes on the motion runtime's loop.
        let t = ui.input(|i| i.time);
        let p = cl::motion::HOTSPOT_PULSE.at(t, 0.0, 0);
        let alpha = 140 + (110.0 * (1.0 - p)) as u8;
        let c = Tone::Coral.color();
        ui.label(
            RichText::new(cl::icon::REC)
                .size(16.0)
                .color(Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha)),
        );
        ui.label(
            RichText::new(format!("REC · {count}"))
                .monospace()
                .color(Tone::Coral.color()),
        );
        if pilot_chip(
            ui,
            STEPS_MARK,
            cl::icon::MARK,
            "Mark",
            TONE,
            "Keep THIS moment as a step (Ctrl+Shift+X from anywhere)",
        ) {
            actions.steps_mark = true;
        }
        if pilot_chip(
            ui,
            STEPS_STOP,
            cl::icon::STOP,
            "Stop",
            TONE,
            "Stop recording and name your guide",
        ) {
            actions.steps_stop = true;
        }
        ui.ctx().request_repaint(); // the pulse never rests while recording
    });
}

/// A BRUTAL palette chip (doctrine): hard offset shadow, thick tone
/// edge, hover presses the chip onto its shadow. The `tip` is LAW —
/// every chip explains itself in layman language.
fn pilot_chip(
    ui: &mut egui::Ui,
    id: PilotId,
    glyph: &str,
    label: &str,
    tone: Tone,
    tip: &str,
) -> bool {
    let text = if label.is_empty() {
        glyph.to_string()
    } else {
        format!("{glyph} {label}")
    };
    let galley =
        ui.painter()
            .layout_no_wrap(text.clone(), FontId::proportional(13.0), cl::ink::BASE);
    let (slot, resp) =
        ui.allocate_exact_size(egui::vec2(galley.size().x + 24.0, 30.0), Sense::click());
    let lift = if resp.hovered() && !resp.is_pointer_button_down_on() {
        egui::Vec2::ZERO
    } else {
        egui::vec2(1.0, 1.0)
    };
    let chip = Rect::from_min_size(slot.min + lift, slot.size() - egui::vec2(3.0, 3.0));
    let shadow = chip.translate(egui::vec2(3.0, 3.0) - lift);
    let painter = ui.painter();
    painter.rect_filled(
        shadow,
        CornerRadius::same(cl::RADIUS_SM),
        Color32::from_black_alpha(230),
    );
    painter.rect_filled(
        chip,
        CornerRadius::same(cl::RADIUS_SM),
        cl::surface::OVERLAY,
    );
    painter.rect_stroke(
        chip,
        CornerRadius::same(cl::RADIUS_SM),
        egui::Stroke::new(2.0, tone.wash()),
        StrokeKind::Inside,
    );
    painter.text(
        chip.center(),
        Align2::CENTER_CENTER,
        text,
        FontId::proportional(13.0),
        cl::ink::BASE,
    );
    ui.pilot_track(
        &resp,
        NodeMetadata::new(
            id,
            PilotRole::Button,
            if label.is_empty() { glyph } else { label },
        ),
    );
    cl::tip(resp, tip).clicked()
}

pub(crate) fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let cut: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{cut}…")
    }
}
