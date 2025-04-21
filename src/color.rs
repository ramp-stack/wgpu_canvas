mod renderer;

pub use renderer::ColorRenderer;

#[derive(Clone, Copy, Debug)]
pub struct Color(pub u8, pub u8, pub u8, pub u8);

impl Color {
    pub fn from_hex(color: &'static str, alpha: u8) -> Self {
        let ce = "Color was not a Hex Value";
        let c = hex::decode(color.strip_prefix('#').unwrap_or(color)).expect(ce);
        Color(c[0], c[1], c[2], alpha)
    }

    pub(crate) fn color(&self) -> [f32; 4] {
        let c = |f: u8| (((f as f32 / u8::MAX as f32) + 0.055) / 1.055).powf(2.4);
        [c(self.0), c(self.1), c(self.2), c(self.3)]
    }
}
