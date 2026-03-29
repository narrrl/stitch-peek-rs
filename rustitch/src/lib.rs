pub mod pes;
mod render;

pub use render::render_thumbnail;

/// Parse a PES file and render a thumbnail PNG of the given size.
pub fn thumbnail(pes_data: &[u8], size: u32) -> Result<Vec<u8>, pes::Error> {
    let design = pes::parse(pes_data)?;
    let resolved = pes::resolve(&design)?;
    let png_bytes = render::render_thumbnail(&resolved, size)?;
    Ok(png_bytes)
}
