pub mod error;
pub mod format;
pub mod palette;
pub mod types;

pub mod pes;

mod render;
mod resolve;

pub use error::Error;
pub use format::Format;
pub use render::render_thumbnail;
pub use types::{BoundingBox, ResolvedDesign, StitchCommand, StitchSegment};

/// Parse a PES file and render a thumbnail PNG of the given size.
pub fn thumbnail(pes_data: &[u8], size: u32) -> Result<Vec<u8>, Error> {
    let design = pes::parse(pes_data)?;
    let resolved = pes::resolve(&design)?;
    render::render_thumbnail(&resolved, size)
}

/// Parse any supported format and render a thumbnail PNG.
pub fn thumbnail_format(data: &[u8], size: u32, fmt: Format) -> Result<Vec<u8>, Error> {
    let resolved = parse_and_resolve(data, fmt)?;
    render::render_thumbnail(&resolved, size)
}

fn parse_and_resolve(data: &[u8], fmt: Format) -> Result<ResolvedDesign, Error> {
    match fmt {
        Format::Pes => {
            let design = pes::parse(data)?;
            pes::resolve(&design)
        }
        _ => Err(Error::UnsupportedFormat),
    }
}
