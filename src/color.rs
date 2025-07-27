mod renderer;

pub use renderer::ColorRenderer;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Color(pub u8, pub u8, pub u8, pub u8);

impl Default for Color {
    fn default() -> Color {
        Color::from_hex("FFFFFF", 255)
    }
}

impl Color {
    pub fn from_hex(color: &str, alpha: u8) -> Self {
        let ce = "Color was not a Hex Value";
        let c = hex::decode(color.strip_prefix('#').unwrap_or(color)).expect(ce);
        Color(c[0], c[1], c[2], alpha)
    }

    pub fn hex(&self) -> String {
        format!("{:02X}{:02X}{:02X}", self.0, self.1, self.2)
    }

    pub fn opacity(&self) -> u8 {
        self.3
    }

    pub(crate) fn color(&self) -> [f32; 4] {
        let c = |f: u8| (((f as f32 / u8::MAX as f32) + 0.055) / 1.055).powf(2.4);
        [c(self.0), c(self.1), c(self.2), c(self.3)]
    }
}
