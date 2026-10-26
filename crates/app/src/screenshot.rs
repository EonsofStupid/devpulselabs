//! The real-window screenshot harness shared by startup and the main app.

use std::path::Path;

/// Returns the screenshot verdict once captured, or on timeout.
pub fn frame(
    ctx: &egui::Context,
    path: Option<&Path>,
    frames: &mut u32,
    target: u32,
) -> Option<bool> {
    let path = path?;
    *frames = frames.saturating_add(1);
    let target = target.max(1);
    if *frames == target {
        ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
    }
    let captured = ctx.input(|input| {
        input.events.iter().find_map(|event| match event {
            egui::Event::Screenshot { image, .. } => Some(image.clone()),
            _ => None,
        })
    });
    if let Some(image) = captured {
        let [width, height] = image.size;
        let rgba: Vec<u8> = image
            .pixels
            .iter()
            .flat_map(|pixel| pixel.to_array())
            .collect();
        return Some(
            match image::RgbaImage::from_raw(width as u32, height as u32, rgba)
                .map(|buffer| buffer.save(path))
            {
                Some(Ok(())) => {
                    eprintln!("screenshot written: {}", path.display());
                    true
                }
                other => {
                    eprintln!("screenshot FAILED: {other:?}");
                    false
                }
            },
        );
    }
    ctx.request_repaint();
    if *frames > target.saturating_add(300) {
        eprintln!("screenshot never arrived after 300 frames");
        return Some(false);
    }
    None
}
