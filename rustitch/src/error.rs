use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid PES magic: expected #PES, got {0:?}")]
    InvalidPesMagic([u8; 4]),
    #[error("file too short: need {expected} bytes, got {actual}")]
    TooShort { expected: usize, actual: usize },
    #[error("invalid PEC offset: {0} exceeds file length {1}")]
    InvalidPecOffset(u32, usize),
    #[error("invalid header: {0}")]
    InvalidHeader(String),
    #[error("no stitch data found")]
    NoStitchData,
    #[error("empty design: no stitch segments produced")]
    EmptyDesign,
    #[error("unsupported format")]
    UnsupportedFormat,
    #[error("render error: {0}")]
    Render(String),
    #[error("PNG encoding error: {0}")]
    PngEncode(#[from] png::EncodingError),
}
