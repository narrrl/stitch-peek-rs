use crate::error::Error;
use crate::types::{ResolvedDesign, StitchCommand};

/// Parse a VP3 (Pfaff/Viking) file from raw bytes.
///
/// VP3 is a hierarchical format:
///   - File header with "%vsm%" magic (or similar signature)
///   - Design metadata section
///   - One or more color sections, each containing:
///     - Thread color (RGB)
///     - Stitch data block
///
/// Byte order: mixed, but length-prefixed strings and section sizes use big-endian.
/// Stitch encoding: variable-length (1 byte for small moves, 3 bytes for large).
pub fn parse(data: &[u8]) -> Result<(Vec<StitchCommand>, Vec<(u8, u8, u8)>), Error> {
    if data.len() < 20 {
        return Err(Error::TooShort {
            expected: 20,
            actual: data.len(),
        });
    }

    let mut reader = Reader::new(data);

    // VP3 files start with a magic/signature section
    // Skip the initial header to find the design data
    // The format starts with a variable-length producer string, then design sections
    skip_vp3_header(&mut reader)?;

    let mut colors = Vec::new();
    let mut commands = Vec::new();

    // Read color sections
    let color_section_count = reader.read_u16_be()?;

    for _ in 0..color_section_count {
        if reader.remaining() < 4 {
            break;
        }

        let color = read_color_section(&mut reader, &mut commands)?;
        colors.push(color);
    }

    if commands.is_empty() {
        return Err(Error::NoStitchData);
    }

    if !matches!(commands.last(), Some(StitchCommand::End)) {
        commands.push(StitchCommand::End);
    }

    Ok((commands, colors))
}

/// Parse a VP3 file and resolve to a renderable design.
pub fn parse_and_resolve(data: &[u8]) -> Result<ResolvedDesign, Error> {
    let (commands, colors) = parse(data)?;
    crate::resolve::resolve(&commands, colors)
}

fn skip_vp3_header(reader: &mut Reader) -> Result<(), Error> {
    // Skip magic/producer string at start
    // VP3 starts with a string like "%vsm%" or similar, followed by metadata
    // Find the start of actual design data by looking for patterns

    // Read and skip the initial producer/signature string
    skip_string(reader)?;

    // Skip design metadata: dimensions and other header fields
    // After the producer string there are typically coordinate fields (i32 BE)
    // and additional metadata strings
    if reader.remaining() < 38 {
        return Err(Error::TooShort {
            expected: 38,
            actual: reader.remaining(),
        });
    }

    // Skip: design size fields (4x i32 = 16 bytes) + unknown bytes (4) + unknown (4)
    reader.skip(24)?;

    // Skip design notes/comments strings
    skip_string(reader)?; // x-offset or notes
    skip_string(reader)?; // y-offset or notes

    // Skip remaining header fields before color sections
    // There are typically 6 more bytes of header data
    if reader.remaining() >= 6 {
        reader.skip(6)?;
    }

    // Skip another potential string
    if reader.remaining() >= 2 {
        let peek = reader.peek_u16_be();
        if let Ok(len) = peek {
            if len < 1000 && (len as usize) + 2 <= reader.remaining() {
                skip_string(reader)?;
            }
        }
    }

    Ok(())
}

fn read_color_section(
    reader: &mut Reader,
    commands: &mut Vec<StitchCommand>,
) -> Result<(u8, u8, u8), Error> {
    // Color change between sections (except first)
    if !commands.is_empty() {
        commands.push(StitchCommand::ColorChange);
    }

    // Skip section start marker/offset bytes
    // Color sections start with coordinate offset data
    if reader.remaining() < 12 {
        return Err(Error::TooShort {
            expected: 12,
            actual: reader.remaining(),
        });
    }

    // Skip section offset/position data (2x i32 = 8 bytes)
    reader.skip(8)?;

    // Skip thread info string
    skip_string(reader)?;

    // Read thread color: RGB (3 bytes)
    if reader.remaining() < 3 {
        return Err(Error::TooShort {
            expected: 3,
            actual: reader.remaining(),
        });
    }
    let r = reader.read_u8()?;
    let g = reader.read_u8()?;
    let b = reader.read_u8()?;

    // Skip remaining thread metadata (thread type, weight, catalog info)
    // Skip to stitch data: look for the stitch count field
    skip_string(reader)?; // thread catalog number
    skip_string(reader)?; // thread description

    // Skip thread brand and additional metadata
    // There's typically some padding/unknown bytes here
    if reader.remaining() >= 18 {
        reader.skip(18)?;
    }

    // Read stitch data
    let stitch_byte_count = if reader.remaining() >= 4 {
        reader.read_u32_be()? as usize
    } else {
        return Ok((r, g, b));
    };

    if stitch_byte_count == 0 || stitch_byte_count > reader.remaining() {
        // Skip what we can
        return Ok((r, g, b));
    }

    let stitch_end = reader.pos + stitch_byte_count;
    decode_vp3_stitches(reader, commands, stitch_end);

    // Ensure we're at the right position after stitch data
    if reader.pos < stitch_end {
        reader.pos = stitch_end;
    }

    Ok((r, g, b))
}

fn decode_vp3_stitches(reader: &mut Reader, commands: &mut Vec<StitchCommand>, end: usize) {
    while reader.pos < end && reader.remaining() >= 2 {
        let b1 = reader.data[reader.pos];

        // Check for 3-byte extended coordinates (high bit set on first byte)
        if b1 & 0x80 != 0 {
            if reader.remaining() < 4 {
                break;
            }
            let dx = read_i16_be(reader.data, reader.pos);
            reader.pos += 2;
            let dy = read_i16_be(reader.data, reader.pos);
            reader.pos += 2;

            // Large moves are jumps
            commands.push(StitchCommand::Jump { dx, dy: -dy });
        } else {
            // 1-byte per coordinate
            let dx = reader.data[reader.pos] as i8 as i16;
            reader.pos += 1;
            if reader.pos >= end {
                break;
            }
            let dy = -(reader.data[reader.pos] as i8 as i16);
            reader.pos += 1;

            if dx == 0 && dy == 0 {
                // Zero-length stitch can be a trim marker
                commands.push(StitchCommand::Trim);
            } else {
                commands.push(StitchCommand::Stitch { dx, dy });
            }
        }
    }
}

fn skip_string(reader: &mut Reader) -> Result<(), Error> {
    if reader.remaining() < 2 {
        return Err(Error::TooShort {
            expected: reader.pos + 2,
            actual: reader.data.len(),
        });
    }
    let len = reader.read_u16_be()? as usize;
    if len > reader.remaining() {
        return Err(Error::InvalidHeader(format!(
            "string length {} exceeds remaining data {}",
            len,
            reader.remaining()
        )));
    }
    reader.skip(len)?;
    Ok(())
}

fn read_i16_be(data: &[u8], pos: usize) -> i16 {
    i16::from_be_bytes([data[pos], data[pos + 1]])
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    fn read_u8(&mut self) -> Result<u8, Error> {
        if self.pos >= self.data.len() {
            return Err(Error::TooShort {
                expected: self.pos + 1,
                actual: self.data.len(),
            });
        }
        let v = self.data[self.pos];
        self.pos += 1;
        Ok(v)
    }

    fn read_u16_be(&mut self) -> Result<u16, Error> {
        if self.pos + 2 > self.data.len() {
            return Err(Error::TooShort {
                expected: self.pos + 2,
                actual: self.data.len(),
            });
        }
        let v = u16::from_be_bytes([self.data[self.pos], self.data[self.pos + 1]]);
        self.pos += 2;
        Ok(v)
    }

    fn peek_u16_be(&self) -> Result<u16, Error> {
        if self.pos + 2 > self.data.len() {
            return Err(Error::TooShort {
                expected: self.pos + 2,
                actual: self.data.len(),
            });
        }
        Ok(u16::from_be_bytes([
            self.data[self.pos],
            self.data[self.pos + 1],
        ]))
    }

    fn read_u32_be(&mut self) -> Result<u32, Error> {
        if self.pos + 4 > self.data.len() {
            return Err(Error::TooShort {
                expected: self.pos + 4,
                actual: self.data.len(),
            });
        }
        let v = u32::from_be_bytes([
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ]);
        self.pos += 4;
        Ok(v)
    }

    fn skip(&mut self, n: usize) -> Result<(), Error> {
        if self.pos + n > self.data.len() {
            return Err(Error::TooShort {
                expected: self.pos + n,
                actual: self.data.len(),
            });
        }
        self.pos += n;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_small_stitch() {
        let mut commands = Vec::new();
        // Two small stitches: (10, -20) and (5, -3)
        let data = [0x0A, 0x14, 0x05, 0x03];
        let mut reader = Reader::new(&data);
        decode_vp3_stitches(&mut reader, &mut commands, data.len());
        assert_eq!(commands.len(), 2);
        assert!(matches!(
            commands[0],
            StitchCommand::Stitch { dx: 10, dy: -20 }
        ));
    }

    #[test]
    fn decode_large_jump() {
        let mut commands = Vec::new();
        // Large move: high bit set, 2-byte BE dx and dy
        // dx = 0x8100 = -32512 as i16, dy = 0x0100 = 256
        let data = [0x81, 0x00, 0x01, 0x00];
        let mut reader = Reader::new(&data);
        decode_vp3_stitches(&mut reader, &mut commands, data.len());
        assert_eq!(commands.len(), 1);
        assert!(matches!(commands[0], StitchCommand::Jump { .. }));
    }
}
