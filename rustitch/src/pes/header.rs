use super::Error;

#[derive(Debug)]
pub struct PesHeader {
    pub version: [u8; 4],
    pub pec_offset: u32,
}

pub fn parse_header(data: &[u8]) -> Result<PesHeader, Error> {
    if data.len() < 12 {
        return Err(Error::TooShort {
            expected: 12,
            actual: data.len(),
        });
    }

    let magic = &data[0..4];
    if magic != b"#PES" {
        let mut m = [0u8; 4];
        m.copy_from_slice(magic);
        return Err(Error::InvalidMagic(m));
    }

    let mut version = [0u8; 4];
    version.copy_from_slice(&data[4..8]);

    let pec_offset = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);

    Ok(PesHeader {
        version,
        pec_offset,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_header() {
        let mut data = vec![0u8; 20];
        data[0..4].copy_from_slice(b"#PES");
        data[4..8].copy_from_slice(b"0001");
        // PEC offset = 16 (little-endian)
        data[8..12].copy_from_slice(&16u32.to_le_bytes());

        let header = parse_header(&data).unwrap();
        assert_eq!(&header.version, b"0001");
        assert_eq!(header.pec_offset, 16);
    }

    #[test]
    fn reject_invalid_magic() {
        let data = b"NOTPES0001\x10\x00\x00\x00";
        let err = parse_header(data).unwrap_err();
        assert!(matches!(err, Error::InvalidMagic(_)));
    }

    #[test]
    fn reject_too_short() {
        let data = b"#PES00";
        let err = parse_header(data).unwrap_err();
        assert!(matches!(err, Error::TooShort { .. }));
    }
}
