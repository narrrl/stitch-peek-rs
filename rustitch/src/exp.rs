use crate::error::Error;
use crate::palette::default_colors;
use crate::types::{ResolvedDesign, StitchCommand};

/// Parse an EXP (Melco) file from raw bytes into stitch commands.
///
/// EXP format: 2 bytes per stitch (signed i8 dx, dy).
/// Escape byte 0x80 followed by a control byte:
///   0x01 = color change
///   0x02 = color change (variant)
///   0x04 = jump (next 2 bytes are jump dx, dy)
///   0x80 = trim
pub fn parse(data: &[u8]) -> Result<Vec<StitchCommand>, Error> {
    if data.len() < 2 {
        return Err(Error::TooShort {
            expected: 2,
            actual: data.len(),
        });
    }

    let mut commands = Vec::new();
    let mut i = 0;

    while i + 1 < data.len() {
        let b1 = data[i];
        let b2 = data[i + 1];

        if b1 == 0x80 {
            match b2 {
                0x01 | 0x02 => {
                    commands.push(StitchCommand::ColorChange);
                    i += 2;
                }
                0x80 => {
                    commands.push(StitchCommand::Trim);
                    i += 2;
                }
                0x04 => {
                    // Jump: next 2 bytes are the movement
                    i += 2;
                    if i + 1 >= data.len() {
                        break;
                    }
                    let dx = data[i] as i8 as i16;
                    let dy = data[i + 1] as i8 as i16;
                    commands.push(StitchCommand::Jump { dx, dy });
                    i += 2;
                }
                _ => {
                    // Unknown escape, skip
                    i += 2;
                }
            }
        } else {
            let dx = b1 as i8 as i16;
            let dy = b2 as i8 as i16;
            commands.push(StitchCommand::Stitch { dx, dy });
            i += 2;
        }
    }

    commands.push(StitchCommand::End);

    if commands.len() <= 1 {
        return Err(Error::NoStitchData);
    }

    Ok(commands)
}

/// Parse an EXP file and resolve to a renderable design.
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
    fn parse_simple_stitches() {
        let data = [0x0A, 0x14, 0x05, 0x03];
        let cmds = parse(&data).unwrap();
        assert!(matches!(
            cmds[0],
            StitchCommand::Stitch { dx: 10, dy: 20 }
        ));
        assert!(matches!(cmds[1], StitchCommand::Stitch { dx: 5, dy: 3 }));
        assert!(matches!(cmds[2], StitchCommand::End));
    }

    #[test]
    fn parse_negative_coords() {
        // -10 as i8 = 0xF6, -20 as i8 = 0xEC
        let data = [0xF6, 0xEC];
        let cmds = parse(&data).unwrap();
        assert!(matches!(
            cmds[0],
            StitchCommand::Stitch { dx: -10, dy: -20 }
        ));
    }

    #[test]
    fn parse_color_change() {
        let data = [0x0A, 0x14, 0x80, 0x01, 0x05, 0x03];
        let cmds = parse(&data).unwrap();
        assert!(matches!(cmds[0], StitchCommand::Stitch { .. }));
        assert!(matches!(cmds[1], StitchCommand::ColorChange));
        assert!(matches!(cmds[2], StitchCommand::Stitch { dx: 5, dy: 3 }));
    }

    #[test]
    fn parse_jump() {
        let data = [0x80, 0x04, 0x0A, 0x14];
        let cmds = parse(&data).unwrap();
        assert!(matches!(
            cmds[0],
            StitchCommand::Jump { dx: 10, dy: 20 }
        ));
    }

    #[test]
    fn parse_trim() {
        let data = [0x0A, 0x14, 0x80, 0x80, 0x05, 0x03];
        let cmds = parse(&data).unwrap();
        assert!(matches!(cmds[1], StitchCommand::Trim));
    }
}
