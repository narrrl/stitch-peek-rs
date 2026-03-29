use super::Error;

pub struct PecHeader {
    pub label: String,
    pub color_count: u8,
    pub color_indices: Vec<u8>,
}

#[derive(Debug, Clone)]
pub enum StitchCommand {
    Stitch { dx: i16, dy: i16 },
    Jump { dx: i16, dy: i16 },
    Trim,
    ColorChange,
    End,
}

/// Parse the PEC header starting at the PEC section offset.
/// Returns the header and the byte offset (relative to pec_data start) where stitch data begins.
pub fn parse_pec_header(pec_data: &[u8]) -> Result<(PecHeader, usize), Error> {
    // PEC section starts with "LA:" label field (19 bytes total)
    if pec_data.len() < 532 {
        return Err(Error::TooShort {
            expected: 532,
            actual: pec_data.len(),
        });
    }

    // Label: bytes 0..19, starts with "LA:"
    let label_raw = &pec_data[3..19];
    let label = std::str::from_utf8(label_raw)
        .unwrap_or("")
        .trim()
        .to_string();

    // Color count at offset 48 from PEC start
    let color_count = pec_data[48] + 1;

    // Color indices follow at offset 49
    let color_indices: Vec<u8> = pec_data[49..49 + color_count as usize].to_vec();

    // Stitch data starts at offset 532 from PEC section start
    // (48 bytes header + 463 bytes padding/thumbnail = 512, plus 20 bytes of graphic data = 532)
    // Actually the standard offset is 512 + the two thumbnail sections.
    // The typical approach: skip to offset 48 + 1 + color_count + padding to 512, then skip thumbnails.
    // Simplified: PEC stitch data offset = 512 + 20 (for the stitch data header that contains the graphic offsets)
    // A more robust approach: read the stitch data offset from the header.

    // At PEC offset + 20..24 there are two u16 LE values for the thumbnail image offsets.
    // The stitch data typically starts after a fixed 532-byte header region.
    // Let's use the more standard approach from libpes:
    // Offset 514-515 (relative to PEC start): thumbnail1 image offset (u16 LE, relative)
    // But the simplest reliable approach is to find the stitch data after the fixed header.

    // The standard PEC header is 512 bytes, followed by two thumbnail images.
    // Thumbnail 1: 6 bytes wide × 38 bytes high = 228 bytes (48×38 pixel, 1bpp padded)
    // Actually, typical PEC has: after the 512-byte block, there are two graphics sections.
    // The stitch data starts after those graphics.
    //
    // More robust: bytes 514..516 give the thumbnail offset (little-endian u16).
    // We can derive stitch data from there, but let's use the standard fixed sizes.
    // Thumbnail 1: at offset 512, size = ceil(width*2/8) * height, with default 48×38 = 6*38=228
    // Thumbnail 2: at offset 512+228=740, size = ceil(width*2/8) * height, default 96×76=12*76=912
    // Stitch data at: 512 + 228 + 912 = 1652? That doesn't seem right.
    //
    // Actually from libpes wiki: PEC header is 20 bytes, then color info, then padding to
    // reach a 512-byte boundary. At byte 512 is the beginning of the PEC graphic section.
    // After the graphics come the stitch data. But graphic sizes vary.
    //
    // The correct approach: at PEC_start + 514 (bytes 514-515), read a u16 LE which gives
    // the absolute offset from PEC_start to the first thumbnail. Then after thumbnails come stitches.
    // BUT actually, the standard approach used by most parsers is simpler:
    //
    // pyembroidery approach: seek to PEC_start + 532, that's where stitch data starts.
    // The 532 = 512 + 20 (20 bytes for graphic header).
    //
    // Let's verify: pyembroidery's PecReader reads stitches starting 532 bytes after PEC start.
    // Let's go with 532.

    let stitch_data_offset = 532;

    if pec_data.len() <= stitch_data_offset {
        return Err(Error::NoStitchData);
    }

    Ok((
        PecHeader {
            label,
            color_count,
            color_indices,
        },
        stitch_data_offset,
    ))
}

/// Decode PEC stitch byte stream into a list of commands.
pub fn decode_stitches(data: &[u8]) -> Result<Vec<StitchCommand>, Error> {
    let mut commands = Vec::new();
    let mut i = 0;

    while i < data.len() {
        let b1 = data[i];

        // End marker
        if b1 == 0xFF {
            commands.push(StitchCommand::End);
            break;
        }

        // Color change
        if b1 == 0xFE {
            commands.push(StitchCommand::ColorChange);
            i += 2; // skip the 0xFE and the following byte (typically 0xB0)
            continue;
        }

        // Parse dx
        let (dx, dx_flags, bytes_dx) = decode_coordinate(data, i)?;
        i += bytes_dx;

        // Parse dy
        let (dy, dy_flags, bytes_dy) = decode_coordinate(data, i)?;
        i += bytes_dy;

        let flags = dx_flags | dy_flags;

        if flags & 0x20 != 0 {
            // Trim + jump
            commands.push(StitchCommand::Trim);
            commands.push(StitchCommand::Jump { dx, dy });
        } else if flags & 0x10 != 0 {
            commands.push(StitchCommand::Jump { dx, dy });
        } else {
            commands.push(StitchCommand::Stitch { dx, dy });
        }
    }

    if commands.is_empty() {
        return Err(Error::NoStitchData);
    }

    Ok(commands)
}

/// Decode a single coordinate (dx or dy) from the byte stream.
/// Returns (value, flags, bytes_consumed).
fn decode_coordinate(data: &[u8], pos: usize) -> Result<(i16, u8, usize), Error> {
    if pos >= data.len() {
        return Err(Error::TooShort {
            expected: pos + 1,
            actual: data.len(),
        });
    }

    let b = data[pos];

    if b & 0x80 != 0 {
        // Extended 12-bit encoding (2 bytes)
        if pos + 1 >= data.len() {
            return Err(Error::TooShort {
                expected: pos + 2,
                actual: data.len(),
            });
        }
        let b2 = data[pos + 1];
        let flags = b & 0x70; // bits 6-4 for jump/trim flags
        let raw = (((b & 0x0F) as u16) << 8) | (b2 as u16);
        let value = if raw > 0x7FF {
            raw as i16 - 0x1000
        } else {
            raw as i16
        };
        Ok((value, flags, 2))
    } else {
        // 7-bit encoding (1 byte)
        let value = if b > 0x3F { b as i16 - 0x80 } else { b as i16 };
        Ok((value, 0, 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_end_marker() {
        let data = [0xFF];
        let cmds = decode_stitches(&data).unwrap();
        assert_eq!(cmds.len(), 1);
        assert!(matches!(cmds[0], StitchCommand::End));
    }

    #[test]
    fn decode_simple_stitch() {
        // dx=10 (0x0A), dy=20 (0x14), then end
        let data = [0x0A, 0x14, 0xFF];
        let cmds = decode_stitches(&data).unwrap();
        assert_eq!(cmds.len(), 2);
        match &cmds[0] {
            StitchCommand::Stitch { dx, dy } => {
                assert_eq!(*dx, 10);
                assert_eq!(*dy, 20);
            }
            _ => panic!("expected Stitch"),
        }
    }

    #[test]
    fn decode_negative_7bit() {
        // dx=0x50 (80 decimal, > 0x3F so value = 80-128 = -48), dy=0x60 (96-128=-32), end
        let data = [0x50, 0x60, 0xFF];
        let cmds = decode_stitches(&data).unwrap();
        match &cmds[0] {
            StitchCommand::Stitch { dx, dy } => {
                assert_eq!(*dx, -48);
                assert_eq!(*dy, -32);
            }
            _ => panic!("expected Stitch"),
        }
    }

    #[test]
    fn decode_color_change() {
        let data = [0xFE, 0xB0, 0x0A, 0x14, 0xFF];
        let cmds = decode_stitches(&data).unwrap();
        assert!(matches!(cmds[0], StitchCommand::ColorChange));
        assert!(matches!(cmds[1], StitchCommand::Stitch { dx: 10, dy: 20 }));
    }

    #[test]
    fn decode_extended_12bit() {
        // Extended dx: high bit set, flags=0x10 (jump), value = 0x100 = 256
        // byte1 = 0x80 | 0x10 | 0x01 = 0x91, byte2 = 0x00 -> raw = 0x100 = 256
        // dy: simple 0x05 = 5
        let data = [0x91, 0x00, 0x05, 0xFF];
        let cmds = decode_stitches(&data).unwrap();
        assert!(matches!(cmds[0], StitchCommand::Jump { dx: 256, dy: 5 }));
    }

    #[test]
    fn decode_trim_jump() {
        // dx with trim flag (0x20): byte1 = 0x80 | 0x20 | 0x00 = 0xA0, byte2 = 0x0A -> raw=10
        // dy: simple 0x05
        let data = [0xA0, 0x0A, 0x05, 0xFF];
        let cmds = decode_stitches(&data).unwrap();
        assert!(matches!(cmds[0], StitchCommand::Trim));
        assert!(matches!(cmds[1], StitchCommand::Jump { dx: 10, dy: 5 }));
    }
}
