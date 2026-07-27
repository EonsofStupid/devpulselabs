//! W10a — thin Studio place form: put a finished seed slide into an
//! existing content episode chapter. Full LandingCard / budget wall = W10b.

use super::SPEC;
use dpl_coldlight::{self as cl, Tone};
use egui::RichText;
use std::path::PathBuf;

/// One validate-clean seed slide offered as a place source.
#[derive(Clone)]
pub(crate) struct PlaceSource {
    pub id: String,
    pub class: &'static str,
    pub dir: PathBuf,
}

/// One content episode destination (show/episode + chapter ids).
#[derive(Clone)]
pub(crate) struct PlaceEpisode {
    pub show: String,
    pub episode: String,
    pub chapters: Vec<(String, String)>, // (id, title)
}

#[derive(Default)]
pub(crate) struct StudioState {
    sources: Option<Vec<PlaceSource>>,
    episodes: Option<Vec<PlaceEpisode>>,
    source_ix: usize,
    episode_ix: usize,
    chapter_ix: usize,
    status: String,
}

impl StudioState {
    fn ensure_loaded(&mut self) {
        if self.sources.is_none() {
            let mut sources = Vec::new();
            for s in dpl_doc::list_seeds() {
                match dpl_doc::load(&s.dir) {
                    Ok(slide) if dpl_doc::validate(&slide).is_empty() => {
                        sources.push(PlaceSource {
                            id: s.id,
                            class: s.class,
                            dir: s.dir,
                        });
                    }
                    _ => {}
                }
            }
            self.sources = Some(sources);
        }
        if self.episodes.is_none() {
            let root = dpl_engine_capture::content_root();
            let mut episodes = Vec::new();
            for (show, episode, dir) in dpl_doc::list_content_episodes(&root) {
                if let Ok(ep) = dpl_doc::load_episode(&dir) {
                    let chapters = ep
                        .chapters
                        .into_iter()
                        .map(|c| (c.id, c.title))
                        .collect::<Vec<_>>();
                    if !chapters.is_empty() {
                        episodes.push(PlaceEpisode {
                            show,
                            episode,
                            chapters,
                        });
                    }
                }
            }
            self.episodes = Some(episodes);
        }
        let n_src = self.sources.as_ref().map(|s| s.len()).unwrap_or(0);
        if n_src > 0 {
            self.source_ix = self.source_ix.min(n_src - 1);
        }
        let n_ep = self.episodes.as_ref().map(|e| e.len()).unwrap_or(0);
        if n_ep > 0 {
            self.episode_ix = self.episode_ix.min(n_ep - 1);
            let n_ch = self.episodes.as_ref().unwrap()[self.episode_ix]
                .chapters
                .len();
            if n_ch > 0 {
                self.chapter_ix = self.chapter_ix.min(n_ch - 1);
            }
        }
    }

    pub(crate) fn ui(&mut self, ui: &mut egui::Ui, fabric: &Option<dpl_fabric::Fabric>) {
        self.ensure_loaded();
        ui.label(
            RichText::new("STUDIO — place a finished slide into a show episode")
                .heading()
                .color(cl::ink::STRONG),
        );
        ui.label(
            RichText::new(
                "Pick a validate-clean seed slide and an existing content episode chapter. \
                 The slide is COPIED (seeds stay pristine). Full budget strip lands in W10b.",
            )
            .small()
            .color(cl::ink::MUTED),
        );
        ui.add_space(cl::space::S3);

        let sources = self.sources.as_ref().cloned().unwrap_or_default();
        let episodes = self.episodes.as_ref().cloned().unwrap_or_default();

        if sources.is_empty() {
            ui.label(
                RichText::new("no finished seed slides — every seed must validate clean")
                    .color(cl::ink::MUTED),
            );
            return;
        }
        if episodes.is_empty() {
            ui.label(
                RichText::new("no content episodes yet — clone a seed from Episodes first (X3)")
                    .color(cl::ink::MUTED),
            );
            return;
        }

        ui.label(
            RichText::new("Finished slide")
                .strong()
                .color(cl::ink::BASE),
        );
        ui.horizontal_wrapped(|ui| {
            for (i, src) in sources.iter().enumerate() {
                let active = i == self.source_ix;
                let chip = cl::tip(
                    ui.label(
                        RichText::new(format!("{}/{}", src.class, src.id))
                            .monospace()
                            .color(if active {
                                SPEC.tone.color()
                            } else {
                                cl::ink::MUTED
                            }),
                    )
                    .interact(egui::Sense::click()),
                    "Finished seed slide to place (copied, never moved)",
                );
                if chip.clicked() {
                    self.source_ix = i;
                }
            }
        });
        ui.add_space(cl::space::S2);

        ui.label(RichText::new("Episode").strong().color(cl::ink::BASE));
        ui.horizontal_wrapped(|ui| {
            for (i, ep) in episodes.iter().enumerate() {
                let active = i == self.episode_ix;
                let chip = cl::tip(
                    ui.label(
                        RichText::new(format!("{}/{}", ep.show, ep.episode))
                            .monospace()
                            .color(if active {
                                SPEC.tone.color()
                            } else {
                                cl::ink::MUTED
                            }),
                    )
                    .interact(egui::Sense::click()),
                    "Existing content episode — place never creates shows",
                );
                if chip.clicked() {
                    self.episode_ix = i;
                    self.chapter_ix = 0;
                }
            }
        });
        ui.add_space(cl::space::S2);

        let ep = &episodes[self.episode_ix];
        ui.label(RichText::new("Chapter").strong().color(cl::ink::BASE));
        ui.horizontal_wrapped(|ui| {
            for (i, (id, title)) in ep.chapters.iter().enumerate() {
                let active = i == self.chapter_ix;
                let chip = cl::tip(
                    ui.label(RichText::new(format!("{id} · {title}")).monospace().color(
                        if active {
                            SPEC.tone.color()
                        } else {
                            cl::ink::MUTED
                        },
                    ))
                    .interact(egui::Sense::click()),
                    "Chapter that receives the slide",
                );
                if chip.clicked() {
                    self.chapter_ix = i;
                }
            }
        });
        ui.add_space(cl::space::S3);

        let place = cl::tip(
            ui.label(
                RichText::new("place slide")
                    .strong()
                    .color(Tone::Amber.color()),
            )
            .interact(egui::Sense::click()),
            "Copy the finished slide into the chapter and update episode.json",
        );
        if place.clicked() {
            let src = &sources[self.source_ix];
            let ep = &episodes[self.episode_ix];
            let (chapter_id, _) = &ep.chapters[self.chapter_ix];
            match dpl_doc::place_slide(
                &dpl_engine_capture::content_root(),
                &src.dir,
                &ep.show,
                &ep.episode,
                chapter_id,
                None,
            ) {
                Ok(placed) => {
                    self.status = format!(
                        "placed {} → {}/{}/{} (chapter {})",
                        src.id, placed.show, placed.episode, placed.slide, placed.chapter
                    );
                    if let Some(f) = fabric {
                        f.publish_build(dpl_fabric::BuildEvent::SlidePlaced {
                            show: placed.show.clone(),
                            episode: placed.episode.clone(),
                            chapter: placed.chapter.clone(),
                            slide: placed.slide.clone(),
                        });
                        f.publish_build(dpl_fabric::BuildEvent::EpisodeChanged {
                            show: placed.show,
                            episode: placed.episode,
                        });
                    }
                    // Destination list may have grown chapter slide counts — keep.
                    self.episodes = None;
                }
                Err(e) => self.status = format!("place refused: {e}"),
            }
        }

        if !self.status.is_empty() {
            ui.add_space(cl::space::S2);
            ui.label(RichText::new(&self.status).small().color(cl::ink::BASE));
        }
    }
}
