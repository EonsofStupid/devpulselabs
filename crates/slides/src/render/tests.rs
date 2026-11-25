use super::refs::{family_of, size_of};
use super::template::slide_of;
use super::*;
use dpl_coldlight::{self as cl};
use egui::Color32;

#[test]
fn every_documented_size_ref_resolves_and_unknowns_degrade() {
    let fluid = cl::Fluid::of(Rect::from_min_size(
        egui::Pos2::ZERO,
        egui::vec2(1280.0, 720.0),
    ));
    for name in [
        "eyebrow",
        "body-sm",
        "body",
        "body-lg",
        "h4",
        "h3",
        "h2",
        "h1",
        "display",
        "display-xl",
    ] {
        assert!(size_of(&fluid, name).is_some(), "size {name} must resolve");
    }
    assert!(size_of(&fluid, "gigantic").is_none());
    assert_eq!(family_of("not-in-the-pool"), egui::FontFamily::Proportional);
    assert_eq!(cl::ink::named("chartreuse", Color32::RED), cl::ink::BASE);
}

/// A text block with an explicit size — the fixtures' common shape.
fn sized(id: &str, text: &str, size: &str) -> dpl_doc::Block {
    dpl_doc::Block::Text {
        id: id.into(),
        preset: None,
        tone: None,
        text: text.into(),
        font: None,
        size: Some(size.into()),
        ink: None,
        enter: None,
        unlinked_from: None,
    }
}

/// A document full of UNKNOWN refs must render without panicking — the
/// degrade law, proven through a real theme'd context (an unregistered
/// font family would panic at layout if `family_of` didn't guard).
#[test]
fn a_document_of_unknown_refs_renders_degraded_not_crashed() {
    let mut slide = dpl_doc::Slide::new("ghost@1");
    slide.background = Some(dpl_doc::Background::MoodBlend {
        moods: vec!["no-such-mood".into(), "also-missing".into()],
        motion: "warp".into(),
        ambience: Some("strobe".into()),
    });
    slide.blocks = vec![dpl_doc::Block::Text {
        id: "t".into(),
        preset: None,
        tone: None,
        text: "still here".into(),
        font: Some("comic-sans".into()),
        size: Some("gigantic".into()),
        ink: Some("chartreuse".into()),
        enter: None,
        unlinked_from: None,
    }];
    slide.enter = Some("rise".into());
    let ctx = egui::Context::default();
    cl::apply_theme(&ctx);
    ctx.run_ui(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let rect = ui.max_rect();
            render(ui, rect, &slide, &dpl_doc::Styling::standalone(), 1.0, 0.0);
        });
    })
    .textures_delta
    .clear();
}

/// The cascade at the paint layer: a slide with NO styling of its own
/// renders through a theme without panicking (W3b).
#[test]
fn a_sparse_slide_renders_with_the_looks_styling() {
    let mut sparse = dpl_doc::Slide::new("title@1");
    // No size either: the paint path falls back to body size.
    sparse.blocks = vec![dpl_doc::Block::text("t", "styled by the show")];
    let ctx = egui::Context::default();
    cl::apply_theme(&ctx);
    ctx.run_ui(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let rect = ui.max_rect();
            render(ui, rect, &sparse, &dpl_doc::Styling::standalone(), 1.0, 0.0);
        });
    })
    .textures_delta
    .clear();
}

/// The beat renderer (D2): every step of a reveal slide paints without
/// panicking, and the RenderMap's block rects are IDENTICAL across
/// steps — reveals never reflow the stack.
#[test]
fn render_step_is_stable_across_every_beat() {
    let mut slide = dpl_doc::Slide::new("title@1");
    slide.blocks = vec![
        sized("title", "Always there", "h1"),
        sized("sub", "Revealed on beat one", "h3"),
    ];
    slide.reveals = vec!["sub".into()];
    assert_eq!(slide.step_count(), 2);
    let styling = dpl_doc::Styling::standalone();
    let ctx = egui::Context::default();
    cl::apply_theme(&ctx);
    let mut maps: Vec<RenderMap> = Vec::new();
    for step in 0..slide.step_count() {
        ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let rect = ui.max_rect();
                maps.push(render_step(ui, rect, &slide, &styling, step, 5.0, 0.0));
            });
        })
        .textures_delta
        .clear();
    }
    let rects = |m: &RenderMap| m.blocks.clone();
    assert_eq!(rects(&maps[0]), rects(&maps[1]), "steps must not reflow");
    assert_eq!(maps[0].blocks.len(), 2, "hidden blocks still hit-test");
}

/// W11: a media slide renders through the ONE path — a full-bleed media
/// backdrop with a text caption on top — without panicking even when the
/// asset is missing (degrade to the labeled plate). The media block owns
/// the whole stage for hit-testing; the caption still stacks as text.
#[test]
fn a_media_slide_renders_full_bleed_and_hit_tests_the_stage() {
    let mut slide = dpl_doc::Slide::new("media@1");
    slide.background = Some(dpl_doc::Background::None);
    slide.blocks = vec![
        dpl_doc::Block::Media {
            id: "shot".into(),
            preset: None,
            tone: None,
            show: "image".into(),
            src: "does/not/exist.png".into(), // degrades to the plate
            fit: Some("cover".into()),
            poster: None,
            enter: None,
            unlinked_from: None,
        },
        sized("cap", "caption over media", "h3"),
    ];
    let ctx = egui::Context::default();
    cl::apply_theme(&ctx);
    let mut map = RenderMap::default();
    ctx.run_ui(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let rect = ui.max_rect();
            map = render_traced(ui, rect, &slide, &dpl_doc::Styling::standalone(), 2.0, 0.0);
            let media_rect = map
                .blocks
                .iter()
                .find(|(id, _)| id == "shot")
                .map(|(_, r)| *r);
            assert_eq!(media_rect, Some(rect), "media hit-tests the full stage");
        });
    })
    .textures_delta
    .clear();
    assert_eq!(map.blocks.len(), 2, "media + caption both present");
}

/// Each line moves by its own entrance (LK3): mid-entrance a rising
/// line is lifted from where it rests, a still line is already there.
#[test]
fn a_line_moves_by_its_own_entrance() {
    let line = |id: &str, enter: &str| {
        let mut block = sized(id, id, "h3");
        if let Some(words) = block.words_mut() {
            *words.enter = Some(enter.into());
        }
        block
    };
    let mut slide = dpl_doc::Slide::new("title@1");
    slide.background = Some(dpl_doc::Background::None);
    slide.blocks = vec![line("still", "none"), line("riser", "rise")];
    // Where each line's words are painted, at `now` seconds after the
    // entrance started.
    let painted_at = |now: f64| -> Vec<f32> {
        let ctx = egui::Context::default();
        cl::apply_theme(&ctx);
        let mut out = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let rect = ui.max_rect();
                render(ui, rect, &slide, &dpl_doc::Styling::standalone(), now, 0.0);
            });
        });
        out.textures_delta.clear();
        out.shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) => Some(text.pos.y),
                _ => None,
            })
            .collect()
    };
    let (mid, rest) = (painted_at(0.05), painted_at(30.0));
    assert_eq!(mid.len(), 2, "both lines paint");
    assert_eq!(mid[0], rest[0], "the still line is already where it rests");
    assert_ne!(mid[1], rest[1], "the rising line is still on its way");
}

/// Shrink to fit (LK3): a long line at a big size stays on the stage —
/// every line's words are painted inside it, none run off the top.
#[test]
fn lines_taller_than_the_stage_shrink_to_fit() {
    let mut slide = dpl_doc::Slide::new("quote@1");
    slide.background = Some(dpl_doc::Background::None);
    slide.blocks = vec![
        sized(
            "quote",
            "The bottleneck was never typing code. It was deciding what to build — and now the machine waits on exactly that.",
            "display",
        ),
        sized("name", "A working developer", "h2"),
    ];
    let stage = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(320.0, 180.0));
    let ctx = egui::Context::default();
    cl::apply_theme(&ctx);
    let mut out = ctx.run_ui(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            render(
                ui,
                stage,
                &slide,
                &dpl_doc::Styling::standalone(),
                30.0,
                0.0,
            );
        });
    });
    out.textures_delta.clear();
    let words: Vec<Rect> = out
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text) => Some(text.galley.rect.translate(text.pos.to_vec2())),
            _ => None,
        })
        .collect();
    // The two lines, and the quote's mark (its layout's decoration).
    assert_eq!(words.len(), 3);
    for rect in words {
        assert!(
            rect.min.y >= stage.min.y - 0.5 && rect.max.y <= stage.max.y + 0.5,
            "{rect:?} runs off {stage:?}"
        );
    }
}

/// Every SEED renders through the real pipeline (registries + theme).
#[test]
fn every_seed_renders_through_the_one_path() {
    let seeds = dpl_doc::list_seeds();
    assert!(!seeds.is_empty(), "seed set missing");
    let ctx = egui::Context::default();
    cl::apply_theme(&ctx);
    for s in seeds {
        let (id, dir) = (s.id, s.dir);
        let slide = dpl_doc::load(&dir).unwrap_or_else(|e| panic!("seed {id}: {e}"));
        ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let rect = ui.max_rect();
                render(ui, rect, &slide, &dpl_doc::Styling::standalone(), 2.0, 0.0);
            });
        })
        .textures_delta
        .clear();
    }
}

/// Render `slide` on a 1280x720 stage and return its map.
fn mapped(slide: &dpl_doc::Slide) -> (Rect, RenderMap) {
    let stage = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1280.0, 720.0));
    let ctx = egui::Context::default();
    cl::apply_theme(&ctx);
    let mut map = None;
    ctx.run_ui(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            map = Some(render_traced(
                ui,
                stage,
                slide,
                &dpl_doc::Styling::standalone(),
                30.0,
                0.0,
            ));
        });
    })
    .textures_delta
    .clear();
    (stage, map.expect("rendered"))
}

/// The component lays the slide out (LS4a): its slots in reading order
/// whatever order the blocks are stored in, and centred when the
/// variant says so.
#[test]
fn a_layout_orders_and_centres_the_lines() {
    let mut slide = dpl_doc::Slide::new("title@1");
    slide.background = Some(dpl_doc::Background::None);
    slide.blocks = vec![
        dpl_doc::Block::text("subtitle", "Under it"),
        dpl_doc::Block::text("title", "The headline"),
        dpl_doc::Block::text("eyebrow", "SHOW NAME"),
    ];
    let (stage, map) = mapped(&slide);
    let ids: Vec<&str> = map.blocks.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(ids, ["eyebrow", "title", "subtitle"]);
    let ys: Vec<f32> = map.blocks.iter().map(|(_, r)| r.min.y).collect();
    assert!(ys.windows(2).all(|w| w[0] < w[1]), "top to bottom: {ys:?}");
    // The title is the big line: the slot's default size.
    assert!(map.blocks[1].1.height() > map.blocks[0].1.height() * 2.0);
    slide.variant.insert("align".into(), "center".into());
    let (_, map) = mapped(&slide);
    for (id, rect) in &map.blocks {
        assert!(
            (rect.center().x - stage.center().x).abs() < 1.0,
            "{id} {rect:?}"
        );
    }
}

/// A slide whose component has no layout flows as it always did: its
/// lines in their own order, on the left.
#[test]
fn a_slide_without_a_layout_flows_in_its_own_order() {
    let mut slide = dpl_doc::Slide::new("freeform@1");
    slide.background = Some(dpl_doc::Background::None);
    slide.blocks = vec![
        sized("b", "Second written, first shown", "h2"),
        sized("a", "First written", "body"),
    ];
    let (stage, map) = mapped(&slide);
    let ids: Vec<&str> = map.blocks.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(ids, ["b", "a"]);
    let left = stage.min.x + cl::Fluid::of(stage).sp_7();
    for (id, rect) in &map.blocks {
        assert!((rect.min.x - left).abs() < 0.5, "{id} {rect:?}");
    }
}

/// A data block is not painted by the flow, so it claims no part of the
/// stage: the background stays selectable on a list card.
#[test]
fn a_data_block_claims_no_part_of_the_stage() {
    let mut slide = dpl_doc::Slide::new("agenda@1");
    slide.background = Some(dpl_doc::Background::None);
    slide.blocks = vec![
        sized("heading", "Today", "h2"),
        dpl_doc::Block::Data {
            id: "items".into(),
            value: serde_json::json!([{"title": "One"}]),
        },
    ];
    let (stage, map) = mapped(&slide);
    assert!(map.blocks.iter().all(|(id, _)| id != "items"));
    assert_eq!(
        map.block_at(stage.right_bottom() - egui::vec2(4.0, 4.0)),
        None
    );
}

/// The layout's decorations place the lines (§4b): a section pins its
/// number to the top and its title to the bottom; a callout's lines sit
/// inside its card, from the card's top.
#[test]
fn the_decorations_place_the_lines() {
    let mut section = dpl_doc::Slide::new("section@1");
    section.background = Some(dpl_doc::Background::None);
    section.blocks = vec![
        dpl_doc::Block::text("title", "The setup"),
        dpl_doc::Block::text("kicker", "Part one"),
        dpl_doc::Block::text("number", "01"),
    ];
    let (stage, map) = mapped(&section);
    let pad = cl::Fluid::of(stage).sp_7();
    let at = |id: &str| map.blocks.iter().find(|(b, _)| b == id).unwrap().1;
    assert!((at("number").min.y - (stage.min.y + pad)).abs() < 0.5);
    assert!((at("title").max.y - (stage.max.y - pad)).abs() < 0.5);
    // The kicker sits above the title, a rule's gap under the number.
    assert!(at("kicker").max.y <= at("title").min.y);
    assert!(at("kicker").min.y > at("number").max.y);

    let mut callout = dpl_doc::Slide::new("callout@1");
    callout.background = Some(dpl_doc::Background::None);
    callout.blocks = vec![
        dpl_doc::Block::text("kicker", "The catch"),
        dpl_doc::Block::text("title", "Free tiers are the demo"),
        dpl_doc::Block::text("body", "Patience is the price."),
    ];
    let (stage, map) = mapped(&callout);
    let card = Rect::from_center_size(
        stage.center(),
        egui::vec2(stage.width() * 0.82, stage.height() * 0.55),
    );
    for (id, rect) in &map.blocks {
        assert!(
            card.shrink(pad - 0.5).contains_rect(*rect),
            "{id} {rect:?} in {card:?}"
        );
    }
    assert!((map.blocks[0].1.min.y - (card.min.y + pad)).abs() < 0.5);
}

/// A template's own values become a slide: content into blocks, the
/// colour into the slide's tone, axes into its variant.
#[test]
fn a_templates_values_become_a_slide() {
    let contract = crate::registry::contract_of("title").expect("the title contract");
    let values = serde_json::json!({
        "tone": "coral", "eyebrow": "SHOW", "title": "Hello", "subtitle": "",
        "align": "center", "media": null
    });
    let slide = slide_of(contract, &values);
    assert_eq!(slide.component, "title@1");
    assert_eq!(slide.tone.as_deref(), Some("coral"));
    assert_eq!(
        slide.variant.get("align").map(String::as_str),
        Some("center")
    );
    assert!(slide.block("title").is_some() && slide.block("eyebrow").is_some());
}
