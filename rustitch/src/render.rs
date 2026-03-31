use tiny_skia::{LineCap, Paint, PathBuilder, Pixmap, Stroke, Transform};

use crate::error::Error;
use crate::types::ResolvedDesign;

/// Render a resolved embroidery design to a PNG image of the given size.
pub fn render_thumbnail(design: &ResolvedDesign, size: u32) -> Result<Vec<u8>, Error> {
    let mut pixmap =
        Pixmap::new(size, size).ok_or_else(|| Error::Render("failed to create pixmap".into()))?;

    let bounds = &design.bounds;
    let design_w = bounds.max_x - bounds.min_x;
    let design_h = bounds.max_y - bounds.min_y;

    if design_w <= 0.0 || design_h <= 0.0 {
        return Err(Error::EmptyDesign);
    }

    let padding = size as f32 * 0.05;
    let available = size as f32 - 2.0 * padding;
    let scale = (available / design_w).min(available / design_h);
    let offset_x = (size as f32 - design_w * scale) / 2.0;
    let offset_y = (size as f32 - design_h * scale) / 2.0;

    let line_width = (scale * 0.3).max(1.0);

    let max_color = design
        .segments
        .iter()
        .map(|s| s.color_index)
        .max()
        .unwrap_or(0);

    for ci in 0..=max_color {
        let (r, g, b) = if ci < design.colors.len() {
            design.colors[ci]
        } else {
            (0, 0, 0)
        };

        let mut paint = Paint::default();
        paint.set_color_rgba8(r, g, b, 255);
        paint.anti_alias = true;

        let stroke = Stroke {
            width: line_width,
            line_cap: LineCap::Round,
            ..Stroke::default()
        };

        let mut pb = PathBuilder::new();
        let mut has_segments = false;

        for seg in &design.segments {
            if seg.color_index != ci {
                continue;
            }
            let sx = (seg.x0 - bounds.min_x) * scale + offset_x;
            let sy = (seg.y0 - bounds.min_y) * scale + offset_y;
            let ex = (seg.x1 - bounds.min_x) * scale + offset_x;
            let ey = (seg.y1 - bounds.min_y) * scale + offset_y;
            pb.move_to(sx, sy);
            pb.line_to(ex, ey);
            has_segments = true;
        }

        if !has_segments {
            continue;
        }

        if let Some(path) = pb.finish() {
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }

    encode_png(&pixmap)
}

/// Encode a tiny-skia Pixmap as a PNG, converting from premultiplied to straight alpha.
fn encode_png(pixmap: &Pixmap) -> Result<Vec<u8>, Error> {
    let width = pixmap.width();
    let height = pixmap.height();
    let src = pixmap.data();

    let mut data = Vec::with_capacity(src.len());
    for chunk in src.chunks_exact(4) {
        let (r, g, b, a) = (chunk[0], chunk[1], chunk[2], chunk[3]);
        if a == 0 {
            data.extend_from_slice(&[0, 0, 0, 0]);
        } else if a == 255 {
            data.extend_from_slice(&[r, g, b, a]);
        } else {
            let af = a as f32;
            data.push((r as f32 * 255.0 / af) as u8);
            data.push((g as f32 * 255.0 / af) as u8);
            data.push((b as f32 * 255.0 / af) as u8);
            data.push(a);
        }
    }

    let mut buf = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut buf, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(&data)?;
    }
    Ok(buf)
}
