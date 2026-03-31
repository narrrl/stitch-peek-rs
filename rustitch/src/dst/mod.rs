use crate::error::Error;
use crate::palette::default_colors;
use crate::types::{ResolvedDesign, StitchCommand};

/// Parse a DST (Tajima) file from raw bytes into stitch commands.
///
/// DST format: 3 bytes per stitch record with bit-packed dx, dy, and control flags.
/// The standard Tajima bit layout maps specific bits across all 3 bytes to coordinate values.
pub fn parse(data: &[u8]) -> Result<Vec<StitchCommand>, Error> {
    if data.len() < 3 {
        return Err(Error::TooShort {
            expected: 3,
            actual: data.len(),
        });
    }

    // DST files may have a 512-byte header; stitch data can start at byte 512
    // if the file is large enough, or at byte 0 for raw stitch streams.
    // The header contains "LA:" at offset 0 if present.
    let offset = if data.len() > 512 && &data[0..3] == b"LA:" {
        512
    } else {
        0
    };

    let stitch_data = &data[offset..];
    if stitch_data.len() < 3 {
        return Err(Error::NoStitchData);
    }

    let mut commands = Vec::new();
    let mut i = 0;

    while i + 2 < stitch_data.len() {
        let b0 = stitch_data[i];
        let b1 = stitch_data[i + 1];
        let b2 = stitch_data[i + 2];
        i += 3;

        // End of file: byte 2 bits 0 and 1 both set, and specific pattern
        if b2 & 0x03 == 0x03 {
            commands.push(StitchCommand::End);
            break;
        }

        let dx = decode_dx(b0, b1, b2);
        let dy = decode_dy(b0, b1, b2);

        // Color change: byte 2 bit 7
        if b2 & 0x80 != 0 {
            commands.push(StitchCommand::ColorChange);
            continue;
        }

        // Jump: byte 2 bit 6
        if b2 & 0x40 != 0 {
            commands.push(StitchCommand::Jump { dx, dy });
            continue;
        }

        commands.push(StitchCommand::Stitch { dx, dy });
    }

    if commands.is_empty() || (commands.len() == 1 && matches!(commands[0], StitchCommand::End)) {
        return Err(Error::NoStitchData);
    }

    // Ensure we have an End marker
    if !matches!(commands.last(), Some(StitchCommand::End)) {
        commands.push(StitchCommand::End);
    }

    Ok(commands)
}

/// Decode X displacement from the 3-byte Tajima record.
/// Standard bit layout for dx across bytes b0, b1, b2.
fn decode_dx(b0: u8, b1: u8, b2: u8) -> i16 {
    let mut x: i16 = 0;
    if b0 & 0x01 != 0 { x += 1; }
    if b0 & 0x02 != 0 { x -= 1; }
    if b0 & 0x04 != 0 { x += 9; }
    if b0 & 0x08 != 0 { x -= 9; }
    if b1 & 0x01 != 0 { x += 3; }
    if b1 & 0x02 != 0 { x -= 3; }
    if b1 & 0x04 != 0 { x += 27; }
    if b1 & 0x08 != 0 { x -= 27; }
    if b2 & 0x04 != 0 { x += 81; }
    if b2 & 0x08 != 0 { x -= 81; }
    x
}

/// Decode Y displacement from the 3-byte Tajima record.
/// Standard bit layout for dy across bytes b0, b1, b2.
fn decode_dy(b0: u8, b1: u8, b2: u8) -> i16 {
    let mut y: i16 = 0;
    if b0 & 0x80 != 0 { y += 1; }
    if b0 & 0x40 != 0 { y -= 1; }
    if b0 & 0x20 != 0 { y += 9; }
    if b0 & 0x10 != 0 { y -= 9; }
    if b1 & 0x80 != 0 { y += 3; }
    if b1 & 0x40 != 0 { y -= 3; }
    if b1 & 0x20 != 0 { y += 27; }
    if b1 & 0x10 != 0 { y -= 27; }
    if b2 & 0x20 != 0 { y += 81; }
    if b2 & 0x10 != 0 { y -= 81; }
    // DST Y axis is inverted (positive = up in machine coords, down in screen coords)
    -y
}

/// Parse a DST file and resolve to a renderable design.
pub fn parse_and_resolve(data: &[u8]) -> Result<ResolvedDesign, Error> {
    let commands = parse(data)?;
    let color_count = commands
        .iter()
        .filter(|c| matches!(c, StitchCommand::ColorChange))
        .count()
        + 1;
    let colors = default_colors(color_count);
    crate::resolve::resolve(&commands, colors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_end_marker() {
        // b2 = 0x03 means end
        let data = [0x00, 0x00, 0x03];
        let cmds = parse(&data).unwrap_err();
        assert!(matches!(cmds, Error::NoStitchData));
    }

    #[test]
    fn decode_simple_stitch() {
        // A normal stitch followed by end
        // dx=+1: b0 bit 0. dy=+1: b0 bit 7. b2=0x00 (normal stitch)
        // Then end marker
        let data = [0x81, 0x00, 0x00, 0x00, 0x00, 0x03];
        let cmds = parse(&data).unwrap();
        assert!(matches!(cmds[0], StitchCommand::Stitch { dx: 1, dy: -1 }));
        assert!(matches!(cmds[1], StitchCommand::End));
    }

    #[test]
    fn decode_jump() {
        // b2 bit 6 = jump
        let data = [0x01, 0x00, 0x40, 0x00, 0x00, 0x03];
        let cmds = parse(&data).unwrap();
        assert!(matches!(cmds[0], StitchCommand::Jump { dx: 1, dy: 0 }));
    }

    #[test]
    fn decode_color_change() {
        // b2 bit 7 = color change
        let data = [0x00, 0x00, 0x80, 0x01, 0x00, 0x00, 0x00, 0x00, 0x03];
        let cmds = parse(&data).unwrap();
        assert!(matches!(cmds[0], StitchCommand::ColorChange));
    }

    #[test]
    fn decode_dx_values() {
        assert_eq!(decode_dx(0x01, 0x00, 0x00), 1);
        assert_eq!(decode_dx(0x02, 0x00, 0x00), -1);
        assert_eq!(decode_dx(0x04, 0x00, 0x00), 9);
        assert_eq!(decode_dx(0x00, 0x04, 0x00), 27);
        assert_eq!(decode_dx(0x00, 0x00, 0x04), 81);
        assert_eq!(decode_dx(0x05, 0x05, 0x04), 1 + 9 + 27 + 81); // 118
    }

    #[test]
    fn decode_dy_values() {
        assert_eq!(decode_dy(0x80, 0x00, 0x00), -1);
        assert_eq!(decode_dy(0x40, 0x00, 0x00), 1);
        assert_eq!(decode_dy(0x20, 0x00, 0x00), -9);
        assert_eq!(decode_dy(0x00, 0x20, 0x00), -27);
        assert_eq!(decode_dy(0x00, 0x00, 0x20), -81);
    }
}
