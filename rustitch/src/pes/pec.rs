use crate::error::Error;
use crate::types::StitchCommand;

pub struct PecHeader {
    pub label: String,
    pub color_count: u8,
    pub color_indices: Vec<u8>,
}

/// Parse the PEC header starting at the PEC section offset.
/// Returns the header and the byte offset (relative to pec_data start) where stitch data begins.
pub fn parse_pec_header(pec_data: &[u8]) -> Result<(PecHeader, usize), Error> {
    if pec_data.len() < 532 {
        return Err(Error::TooShort {
            expected: 532,
            actual: pec_data.len(),
        });
    }

    let label_raw = &pec_data[3..19];
    let label = std::str::from_utf8(label_raw)
        .unwrap_or("")
        .trim()
        .to_string();

    let color_count = pec_data[48] + 1;
    let color_indices: Vec<u8> = pec_data[49..49 + color_count as usize].to_vec();

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

        if b1 == 0xFF {
            commands.push(StitchCommand::End);
            break;
        }

        if b1 == 0xFE {
            commands.push(StitchCommand::ColorChange);
            i += 2;
            continue;
        }

        let (dx, dx_flags, bytes_dx) = decode_coordinate(data, i)?;
        i += bytes_dx;

        let (dy, dy_flags, bytes_dy) = decode_coordinate(data, i)?;
        i += bytes_dy;

        let flags = dx_flags | dy_flags;

        if flags & 0x20 != 0 {
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
        if pos + 1 >= data.len() {
            return Err(Error::TooShort {
                expected: pos + 2,
                actual: data.len(),
            });
        }
        let b2 = data[pos + 1];
        let flags = b & 0x70;
        let raw = (((b & 0x0F) as u16) << 8) | (b2 as u16);
        let value = if raw > 0x7FF {
            raw as i16 - 0x1000
        } else {
            raw as i16
        };
        Ok((value, flags, 2))
    } else {
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
        let data = [0x91, 0x00, 0x05, 0xFF];
        let cmds = decode_stitches(&data).unwrap();
        assert!(matches!(cmds[0], StitchCommand::Jump { dx: 256, dy: 5 }));
    }

    #[test]
    fn decode_trim_jump() {
        let data = [0xA0, 0x0A, 0x05, 0xFF];
        let cmds = decode_stitches(&data).unwrap();
        assert!(matches!(cmds[0], StitchCommand::Trim));
        assert!(matches!(cmds[1], StitchCommand::Jump { dx: 10, dy: 5 }));
    }
}
