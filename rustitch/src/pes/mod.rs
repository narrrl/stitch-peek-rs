mod header;
mod palette;
mod pec;

pub use header::PesHeader;
pub use palette::PEC_PALETTE;
pub use pec::{PecHeader, StitchCommand};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid PES magic: expected #PES, got {0:?}")]
    InvalidMagic([u8; 4]),
    #[error("file too short: need {expected} bytes, got {actual}")]
    TooShort { expected: usize, actual: usize },
    #[error("invalid PEC offset: {0} exceeds file length {1}")]
    InvalidPecOffset(u32, usize),
    #[error("no stitch data found")]
    NoStitchData,
    #[error("empty design: no stitch segments produced")]
    EmptyDesign,
    #[error("render error: {0}")]
    Render(String),
    #[error("PNG encoding error: {0}")]
    PngEncode(#[from] png::EncodingError),
}

pub struct PesDesign {
    pub header: PesHeader,
    pub pec_header: PecHeader,
    pub commands: Vec<StitchCommand>,
}

pub struct StitchSegment {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub color_index: usize,
}

pub struct BoundingBox {
    pub min_x: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
}

pub struct ResolvedDesign {
    pub segments: Vec<StitchSegment>,
    pub colors: Vec<(u8, u8, u8)>,
    pub bounds: BoundingBox,
}

/// Parse a PES file from raw bytes.
pub fn parse(data: &[u8]) -> Result<PesDesign, Error> {
    let header = header::parse_header(data)?;
    let pec_offset = header.pec_offset as usize;

    if pec_offset >= data.len() {
        return Err(Error::InvalidPecOffset(header.pec_offset, data.len()));
    }

    let pec_data = &data[pec_offset..];
    let (pec_header, stitch_data_offset) = pec::parse_pec_header(pec_data)?;
    let commands = pec::decode_stitches(&pec_data[stitch_data_offset..])?;

    Ok(PesDesign {
        header,
        pec_header,
        commands,
    })
}

/// Convert parsed commands into renderable segments with absolute coordinates.
pub fn resolve(design: &PesDesign) -> Result<ResolvedDesign, Error> {
    let mut segments = Vec::new();
    let mut x: f32 = 0.0;
    let mut y: f32 = 0.0;
    let mut color_idx: usize = 0;
    let mut pen_down = true;

    for cmd in &design.commands {
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

    // Compute bounding box
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

    // Resolve colors from palette indices
    let colors: Vec<(u8, u8, u8)> = design
        .pec_header
        .color_indices
        .iter()
        .map(|&idx| {
            let i = (idx as usize).min(PEC_PALETTE.len() - 1);
            PEC_PALETTE[i]
        })
        .collect();

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
