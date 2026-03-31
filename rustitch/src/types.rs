#[derive(Debug, Clone)]
pub enum StitchCommand {
    Stitch { dx: i16, dy: i16 },
    Jump { dx: i16, dy: i16 },
    Trim,
    ColorChange,
    End,
}

pub struct StitchSegment {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub color_index: usize,
}

pub struct BoundingBox {
    pub min_x: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
}

pub struct ResolvedDesign {
    pub segments: Vec<StitchSegment>,
    pub colors: Vec<(u8, u8, u8)>,
    pub bounds: BoundingBox,
}
