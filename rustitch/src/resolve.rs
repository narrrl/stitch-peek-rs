use crate::error::Error;
use crate::types::{BoundingBox, ResolvedDesign, StitchCommand, StitchSegment};

/// Convert parsed stitch commands into renderable segments with absolute coordinates.
pub fn resolve(
    commands: &[StitchCommand],
    colors: Vec<(u8, u8, u8)>,
) -> Result<ResolvedDesign, Error> {
    let mut segments = Vec::new();
    let mut x: f32 = 0.0;
    let mut y: f32 = 0.0;
    let mut color_idx: usize = 0;
    let mut pen_down = true;

    for cmd in commands {
        match cmd {
            StitchCommand::Stitch { dx, dy } => {
                let nx = x + *dx as f32;
                let ny = y + *dy as f32;
                if pen_down {
                    segments.push(StitchSegment {
                        x0: x,
                        y0: y,
                        x1: nx,
                        y1: ny,
                        color_index: color_idx,
                    });
                }
                x = nx;
                y = ny;
                pen_down = true;
            }
            StitchCommand::Jump { dx, dy } => {
                x += *dx as f32;
                y += *dy as f32;
                pen_down = false;
            }
            StitchCommand::Trim => {
                pen_down = false;
            }
            StitchCommand::ColorChange => {
                color_idx += 1;
                pen_down = false;
            }
            StitchCommand::End => break,
        }
    }

    if segments.is_empty() {
        return Err(Error::EmptyDesign);
    }

    let mut min_x = f32::MAX;
    let mut max_x = f32::MIN;
    let mut min_y = f32::MAX;
    let mut max_y = f32::MIN;

    for seg in &segments {
        min_x = min_x.min(seg.x0).min(seg.x1);
        max_x = max_x.max(seg.x0).max(seg.x1);
        min_y = min_y.min(seg.y0).min(seg.y1);
        max_y = max_y.max(seg.y0).max(seg.y1);
    }

    Ok(ResolvedDesign {
        segments,
        colors,
        bounds: BoundingBox {
            min_x,
            max_x,
            min_y,
            max_y,
        },
    })
}
