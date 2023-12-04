#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn decode(index: u8) -> Self {
        PALETTE[index as usize]
    }
}

const PALETTE: [Color; 64] = {
    let colors = include_bytes!("../../ntsc.pal");
    let mut result = [Color::new(0, 0, 0); 64];
    let mut i = 0;
    while i < colors.len() / 3 {
        let r = colors[i * 3];
        let g = colors[i * 3 + 1];
        let b = colors[i * 3 + 2];
        result[i] = Color::new(r, g, b);
        i += 1;
    }
    result
};
