mod palette;

use crate::error::Error;
use crate::types::{ResolvedDesign, StitchCommand};
use palette::JEF_PALETTE;

/// Parse a JEF (Janome) file from raw bytes into stitch commands and color info.
///
/// JEF header layout (little-endian):
///   0..4:   stitch data offset (u32)
///   4..8:   flags/format indicator
///   24..28: color count (u32)
///   28..32: stitch count (u32)
///   116+:   color table (each entry: i32 palette index)
///
/// Stitch data: 2 bytes per stitch (signed i8 dx, dy).
/// Control codes: 0x80 0x01 = color change, 0x80 0x02 = jump, 0x80 0x10 = end.
pub fn parse(data: &[u8]) -> Result<(Vec<StitchCommand>, Vec<(u8, u8, u8)>), Error> {
    if data.len() < 116 {
        return Err(Error::TooShort {
            expected: 116,
            actual: data.len(),
        });
    }

    let stitch_offset = read_u32_le(data, 0) as usize;
    let color_count = read_u32_le(data, 24) as usize;

    if stitch_offset > data.len() {
        return Err(Error::InvalidHeader(format!(
            "stitch data offset {} exceeds file length {}",
            stitch_offset,
            data.len()
        )));
    }

    // Read color table starting at offset 116
    let color_table_start = 116;
    let mut colors = Vec::with_capacity(color_count);
    for i in 0..color_count {
        let entry_offset = color_table_start + i * 4;
        if entry_offset + 4 > data.len() {
            break;
        }
        let idx = read_i32_le(data, entry_offset);
        let palette_idx = if idx >= 0 && (idx as usize) < JEF_PALETTE.len() {
            idx as usize
        } else {
            0
        };
        colors.push(JEF_PALETTE[palette_idx]);
    }

    if colors.is_empty() {
        colors.push((0, 0, 0));
    }

    // Parse stitch data
    let stitch_data = &data[stitch_offset..];
    let commands = decode_stitches(stitch_data)?;

    Ok((commands, colors))
}

fn decode_stitches(data: &[u8]) -> Result<Vec<StitchCommand>, Error> {
    let mut commands = Vec::new();
    let mut i = 0;

    while i + 1 < data.len() {
        let b1 = data[i];
        let b2 = data[i + 1];

        if b1 == 0x80 {
            match b2 {
                0x01 => {
                    commands.push(StitchCommand::ColorChange);
                    i += 2;
                }
                0x02 => {
                    // Jump: next 2 bytes are movement
                    i += 2;
                    if i + 1 >= data.len() {
                        break;
                    }
                    let dx = data[i] as i8 as i16;
                    let dy = -(data[i + 1] as i8 as i16);
                    commands.push(StitchCommand::Jump { dx, dy });
                    i += 2;
                }
                0x10 => {
                    commands.push(StitchCommand::End);
                    break;
                }
                _ => {
                    i += 2;
                }
            }
        } else {
            let dx = b1 as i8 as i16;
            let dy = -(b2 as i8 as i16);
            commands.push(StitchCommand::Stitch { dx, dy });
            i += 2;
        }
    }

    if commands.is_empty() {
        return Err(Error::NoStitchData);
    }

    if !matches!(commands.last(), Some(StitchCommand::End)) {
        commands.push(StitchCommand::End);
    }

    Ok(commands)
}

/// Parse a JEF file and resolve to a renderable design.
pub fn parse_and_resolve(data: &[u8]) -> Result<ResolvedDesign, Error> {
    let (commands, colors) = parse(data)?;
    crate::resolve::resolve(&commands, colors)
}

fn read_u32_le(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

fn read_i32_le(data: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_simple_stitches() {
        let data = [0x0A, 0x14, 0x05, 0x03, 0x80, 0x10];
        let cmds = decode_stitches(&data).unwrap();
        assert!(matches!(
            cmds[0],
            StitchCommand::Stitch { dx: 10, dy: -20 }
        ));
        assert!(matches!(cmds[1], StitchCommand::Stitch { dx: 5, dy: -3 }));
        assert!(matches!(cmds[2], StitchCommand::End));
    }

    #[test]
    fn decode_color_change() {
        let data = [0x0A, 0x14, 0x80, 0x01, 0x05, 0x03, 0x80, 0x10];
        let cmds = decode_stitches(&data).unwrap();
        assert!(matches!(cmds[1], StitchCommand::ColorChange));
    }

    #[test]
    fn decode_jump() {
        let data = [0x80, 0x02, 0x0A, 0x14, 0x80, 0x10];
        let cmds = decode_stitches(&data).unwrap();
        assert!(matches!(
            cmds[0],
            StitchCommand::Jump { dx: 10, dy: -20 }
        ));
    }
}
