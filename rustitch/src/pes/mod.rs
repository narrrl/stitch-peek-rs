mod header;
mod pec;

pub use header::PesHeader;
pub use pec::PecHeader;

// Re-export shared types for backward compatibility
pub use crate::error::Error;
pub use crate::palette::PEC_PALETTE;
pub use crate::types::{BoundingBox, ResolvedDesign, StitchCommand, StitchSegment};

pub struct PesDesign {
    pub header: PesHeader,
    pub pec_header: PecHeader,
    pub commands: Vec<StitchCommand>,
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

/// Convert parsed PES design into renderable segments with absolute coordinates.
pub fn resolve(design: &PesDesign) -> Result<ResolvedDesign, Error> {
    let colors: Vec<(u8, u8, u8)> = design
        .pec_header
        .color_indices
        .iter()
        .map(|&idx| {
            let i = (idx as usize).min(PEC_PALETTE.len() - 1);
            PEC_PALETTE[i]
        })
        .collect();

    crate::resolve::resolve(&design.commands, colors)
}
