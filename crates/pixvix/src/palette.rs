//! The palette's paint layer — tabs, chips, the REC pill, and the C2
//! forced-metadata Save card. Immediate viewports PAINT; every mutation
//! rides `PaletteActions` back to the owner in `lib.rs` (the Save card's
//! text fields are the one exception: they edit the `PendingGuide` the
//! owner lends for the frame).

use dpl_coldlight::{self as cl, Tone};
use egui::{Color32, CornerRadius, RichText};
use egui_pilot_ai::protocol::Role as PilotRole;
use egui_pilot_ai::{NodeMetadata, PilotId, UiPilotExt};

use crate::{PendingGuide, TONE};

const PALETTE_TITLE: PilotId = PilotId::new("pixvix.title");
const PALETTE_CLOSE: PilotId = PilotId::new("pixvix.close");
const TAB_IMG: PilotId = PilotId::new("pixvix.tab.img");
const TAB_VIDEO: PilotId = PilotId::new("pixvix.tab.video");
const TAB_SESSION: PilotId = PilotId::new("pixvix.tab.session");
const TAB_STEPS: PilotId = PilotId::new("pixvix.tab.steps");
const REGION_OPEN: PilotId = PilotId::new("pixvix.region.open");
const SNAP_FULL: PilotId = PilotId::new("pixvix.snap.full");
const FREEFORM: PilotId = PilotId::new("pixvix.freeform");
const SNAP_WINDOW: PilotId = PilotId::new("pixvix.snap.window");
const REFRESH_WINDOWS: PilotId = PilotId::new("pixvix.windows.refresh");
const VIDEO_START: PilotId = PilotId::new("pixvix.video.start");
const VIDEO_STOP: PilotId = PilotId::new("pixvix.video.stop");
const VIDEO_TARGET: PilotId = PilotId::new("pixvix.video.target");
const VIDEO_FORMAT: PilotId = PilotId::new("pixvix.video.format");
const VIDEO_AUDIO: PilotId = PilotId::new("pixvix.video.audio");
const VIDEO_CAP: PilotId = PilotId::new("pixvix.video.cap");
const SESSION_START: PilotId = PilotId::new("pixvix.session.start");
const SESSION_STOP: PilotId = PilotId::new("pixvix.session.stop");
const STEPS_START: PilotId = PilotId::new("pixvix.steps.start");
const STEPS_MARK: PilotId = PilotId::new("pixvix.steps.mark");
const STEPS_STOP: PilotId = PilotId::new("pixvix.steps.stop");
const DELAY_CYCLE: PilotId = PilotId::new("pixvix.delay");
const RESNIP: PilotId = PilotId::new("pixvix.resnip");
const GUIDE_NAME: PilotId = PilotId::new("pixvix.guide.name");
const GUIDE_TAGS: PilotId = PilotId::new("pixvix.guide.tags");
const GUIDE_DEST: PilotId = PilotId::new("pixvix.guide.dest");
const GUIDE_SAVE: PilotId = PilotId::new("pixvix.guide.save");
const GUIDE_DISCARD: PilotId = PilotId::new("pixvix.guide.discard");

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
        let title = ui.label(RichText::new("PixVix").strong().color(cl::ink::STRONG));
        ui.pilot_track(
            &title,
            NodeMetadata::new(PALETTE_TITLE, PilotRole::Heading, "PixVix"),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if pilot_chip(ui, PALETTE_CLOSE, cl::icon::CLOSE, "", TONE) {
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
            let response = ui.selectable_label(active, rich);
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
                if pilot_chip(ui, REGION_OPEN, cl::icon::REGION, "Region", TONE) {
                    actions.begin_region = true;
                }
                if pilot_chip(ui, SNAP_FULL, cl::icon::FULL, "Full", TONE) {
                    actions.snap_full = true;
                }
                if pilot_chip(ui, FREEFORM, cl::icon::FREE, "Free", TONE) {
                    actions.begin_free = true;
                }
                // The delay chip cycles 0 → 3 → 10 seconds (C1).
                let delay_label = if delay == 0 {
                    "0s".to_string()
                } else {
                    format!("{delay}s")
                };
                if pilot_chip(ui, DELAY_CYCLE, "", &delay_label, TONE) {
                    actions.cycle_delay = true;
                }
                if pilot_chip(ui, RESNIP, "", "Re-snip", TONE) {
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
                egui::ComboBox::from_id_salt("pixvix-window")
                    .width(220.0)
                    .selected_text(RichText::new(truncate(&current, 30)).small())
                    .show_ui(ui, |ui| {
                        for (i, w) in windows.iter().enumerate() {
                            if ui.selectable_label(i == picked, truncate(w, 44)).clicked() {
                                actions.pick_window = Some(i);
                            }
                        }
                    });
                if pilot_chip(ui, SNAP_WINDOW, cl::icon::SNIP, "Snap", TONE) {
                    actions.snap_window = true;
                }
                if ui.pilot_button(REFRESH_WINDOWS, "↻").clicked() {
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
                    if pilot_chip(ui, VIDEO_STOP, cl::icon::STOP, "Stop", TONE) {
                        actions.video_stop = true;
                    }
                });
            }
            None => {
                // The C3 matrix: cycling chips (touch law) + the device
                // list when a mode records audio.
                ui.horizontal(|ui| {
                    let target = if video_window { "Window" } else { "Screen" };
                    if pilot_chip(ui, VIDEO_TARGET, "", target, TONE) {
                        actions.toggle_video_target = true;
                    }
                    if pilot_chip(ui, VIDEO_FORMAT, "", format_label, TONE) {
                        actions.cycle_format = true;
                    }
                    if pilot_chip(ui, VIDEO_AUDIO, "", audio_label, TONE) {
                        actions.cycle_audio = true;
                    }
                    if pilot_chip(ui, VIDEO_CAP, "", cap_label, TONE) {
                        actions.cycle_cap = true;
                    }
                });
                if !audio_silent {
                    ui.horizontal(|ui| {
                        let current = audio_devices
                            .get(picked_audio)
                            .cloned()
                            .unwrap_or_else(|| "no audio devices found".into());
                        egui::ComboBox::from_id_salt("pixvix-audio-device")
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
                        if ui.button("↻").clicked() {
                            actions.refresh_audio = true;
                        }
                    });
                }
                ui.horizontal(|ui| {
                    if pilot_chip(ui, VIDEO_START, cl::icon::REC, "Record", TONE) {
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
                    if pilot_chip(ui, SESSION_STOP, cl::icon::STOP, "Stop", TONE) {
                        actions.session_stop = true;
                    }
                });
            }
            None => {
                ui.horizontal(|ui| {
                    if pilot_chip(ui, SESSION_START, cl::icon::REC, "Record session", TONE) {
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
                    if pilot_chip(ui, STEPS_START, cl::icon::REC, "Start guide", TONE) {
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
        if pilot_chip(ui, GUIDE_SAVE, "", "Save", TONE) {
            actions.guide_save = true;
        }
        if pilot_chip(ui, GUIDE_DISCARD, "", "Discard", TONE) {
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
        if pilot_chip(ui, STEPS_MARK, cl::icon::MARK, "Mark", TONE) {
            actions.steps_mark = true;
        }
        if pilot_chip(ui, STEPS_STOP, cl::icon::STOP, "Stop", TONE) {
            actions.steps_stop = true;
        }
        ui.ctx().request_repaint(); // the pulse never rests while recording
    });
}

fn pilot_chip(ui: &mut egui::Ui, id: PilotId, glyph: &str, label: &str, tone: Tone) -> bool {
    let text = if label.is_empty() {
        glyph.to_string()
    } else {
        format!("{glyph} {label}")
    };
    let response = ui.add(
        egui::Button::new(RichText::new(text).color(cl::ink::BASE))
            .fill(cl::surface::OVERLAY)
            .stroke(egui::Stroke::new(1.0, tone.wash()))
            .corner_radius(CornerRadius::same(cl::RADIUS_SM)),
    );
    ui.pilot_track(
        &response,
        NodeMetadata::new(
            id,
            PilotRole::Button,
            if label.is_empty() { glyph } else { label },
        ),
    );
    response.clicked()
}

pub(crate) fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let cut: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{cut}…")
    }
}
