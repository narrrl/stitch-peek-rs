use rustitch::pes::{self, StitchCommand};

const GNOME_BARFS: &[u8] = include_bytes!("fixtures/JLS_Gnome Barfs.PES");
const URSULA_ONE: &[u8] = include_bytes!("fixtures/UrsulaOne.PES");
const UFRONT: &[u8] = include_bytes!("fixtures/UFront.PES");

// -- Header parsing ----------------------------------------------------------

#[test]
fn parse_header_gnome_barfs() {
    let design = pes::parse(GNOME_BARFS).unwrap();
    assert_eq!(&design.header.version, b"0100");
}

#[test]
fn parse_header_ursula_one() {
    let design = pes::parse(URSULA_ONE).unwrap();
    assert_eq!(&design.header.version, b"0060");
}

#[test]
fn parse_header_ufront() {
    let design = pes::parse(UFRONT).unwrap();
    assert_eq!(&design.header.version, b"0060");
}

// -- PEC color table ---------------------------------------------------------

#[test]
fn gnome_barfs_has_colors() {
    let design = pes::parse(GNOME_BARFS).unwrap();
    assert!(
        design.pec_header.color_count > 0,
        "expected at least one color"
    );
    assert_eq!(
        design.pec_header.color_indices.len(),
        design.pec_header.color_count as usize
    );
}

#[test]
fn ursula_one_has_colors() {
    let design = pes::parse(URSULA_ONE).unwrap();
    assert!(design.pec_header.color_count > 0);
    // All color indices should be valid palette entries (0..65)
    for &idx in &design.pec_header.color_indices {
        assert!(
            (idx as usize) < pes::PEC_PALETTE.len(),
            "color index {idx} out of palette range"
        );
    }
}

// -- Stitch commands ---------------------------------------------------------

#[test]
fn gnome_barfs_commands_end_properly() {
    let design = pes::parse(GNOME_BARFS).unwrap();
    let last = design.commands.last().unwrap();
    assert!(
        matches!(last, StitchCommand::End),
        "expected End command as last, got {last:?}"
    );
}

#[test]
fn ursula_one_commands_end_properly() {
    let design = pes::parse(URSULA_ONE).unwrap();
    let last = design.commands.last().unwrap();
    assert!(
        matches!(last, StitchCommand::End),
        "expected End command as last, got {last:?}"
    );
}

#[test]
fn ufront_has_stitches() {
    let design = pes::parse(UFRONT).unwrap();
    let stitch_count = design
        .commands
        .iter()
        .filter(|c| matches!(c, StitchCommand::Stitch { .. }))
        .count();
    assert!(
        stitch_count > 100,
        "expected many stitches, got {stitch_count}"
    );
}

#[test]
fn gnome_barfs_has_color_changes() {
    let design = pes::parse(GNOME_BARFS).unwrap();
    let changes = design
        .commands
        .iter()
        .filter(|c| matches!(c, StitchCommand::ColorChange))
        .count();
    // Multi-color design should have at least one color change
    assert!(changes > 0, "expected color changes, got none");
}

// -- Resolve to segments -----------------------------------------------------

#[test]
fn resolve_gnome_barfs() {
    let design = pes::parse(GNOME_BARFS).unwrap();
    let resolved = pes::resolve(&design).unwrap();

    assert!(!resolved.segments.is_empty());
    assert!(!resolved.colors.is_empty());

    // Bounding box should be non-degenerate
    assert!(resolved.bounds.max_x > resolved.bounds.min_x);
    assert!(resolved.bounds.max_y > resolved.bounds.min_y);
}

#[test]
fn resolve_ursula_one() {
    let design = pes::parse(URSULA_ONE).unwrap();
    let resolved = pes::resolve(&design).unwrap();

    assert!(!resolved.segments.is_empty());
    assert!(resolved.bounds.max_x > resolved.bounds.min_x);
    assert!(resolved.bounds.max_y > resolved.bounds.min_y);
}

#[test]
fn resolve_ufront() {
    let design = pes::parse(UFRONT).unwrap();
    let resolved = pes::resolve(&design).unwrap();

    assert!(!resolved.segments.is_empty());

    // All segment color indices should be within the resolved color list
    let max_ci = resolved
        .segments
        .iter()
        .map(|s| s.color_index)
        .max()
        .unwrap();
    assert!(
        max_ci < resolved.colors.len(),
        "segment references color {max_ci} but only {} colors resolved",
        resolved.colors.len()
    );
}

// -- Full thumbnail pipeline -------------------------------------------------

#[test]
fn thumbnail_gnome_barfs_128() {
    let png = rustitch::thumbnail(GNOME_BARFS, 128).unwrap();
    assert_png_dimensions(&png, 128, 128);
}

#[test]
fn thumbnail_ursula_one_256() {
    let png = rustitch::thumbnail(URSULA_ONE, 256).unwrap();
    assert_png_dimensions(&png, 256, 256);
}

#[test]
fn thumbnail_ufront_64() {
    let png = rustitch::thumbnail(UFRONT, 64).unwrap();
    assert_png_dimensions(&png, 64, 64);
}

#[test]
fn thumbnail_gnome_barfs_not_blank() {
    let png = rustitch::thumbnail(GNOME_BARFS, 128).unwrap();
    let pixels = decode_png_pixels(&png);
    // At least some pixels should have non-zero alpha (not fully transparent)
    let opaque_count = pixels.chunks_exact(4).filter(|px| px[3] > 0).count();
    assert!(
        opaque_count > 100,
        "thumbnail looks blank, only {opaque_count} non-transparent pixels"
    );
}

// -- Helpers -----------------------------------------------------------------

fn assert_png_dimensions(png_data: &[u8], expected_w: u32, expected_h: u32) {
    let decoder = png::Decoder::new(png_data);
    let reader = decoder.read_info().unwrap();
    let info = reader.info();
    assert_eq!(info.width, expected_w, "unexpected PNG width");
    assert_eq!(info.height, expected_h, "unexpected PNG height");
    assert_eq!(info.color_type, png::ColorType::Rgba);
    assert_eq!(info.bit_depth, png::BitDepth::Eight);
}

fn decode_png_pixels(png_data: &[u8]) -> Vec<u8> {
    let decoder = png::Decoder::new(png_data);
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0u8; reader.output_buffer_size()];
    reader.next_frame(&mut buf).unwrap();
    buf
}
