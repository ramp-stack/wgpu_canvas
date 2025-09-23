mod renderer;

pub use renderer::ColorRenderer;

/// RGBA representation of color.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Color(pub u8, pub u8, pub u8, pub u8);

impl Default for Color {
    fn default() -> Color {
        Color::from_hex("FFFFFF", 255)
    }
}

impl Color {
    pub const WHITE: Color = Color(255, 255, 255, 255);
    pub const BLACK: Color = Color(0, 0, 0, 255);
    pub const TRANSPARENT: Color = Color(0, 0, 0, 0);

    /// Creates an RGBA color from a hex code and alpha value.
    ///
    /// ```rust
    /// let pink = Color::from_hex("#FF006E", 255);
    /// ```
    pub fn from_hex(color: &str, alpha: u8) -> Self {
        let ce = "Color was not a Hex Value";
        let c = hex::decode(color.strip_prefix('#').unwrap_or(color)).expect(ce);
        Color(c[0], c[1], c[2], alpha)
    }

    /// Returns the RGBA values as a hex string.
    pub fn hex(&self) -> String {
        format!("{:02X}{:02X}{:02X}", self.0, self.1, self.2)
    }

    /// Returns the opacity value.
    pub fn opacity(&self) -> u8 {
        self.3
    }

    pub(crate) fn color(&self) -> [f32; 4] {
        let c = |f: u8| (((f as f32 / u8::MAX as f32) + 0.055) / 1.055).powf(2.4);
        [c(self.0), c(self.1), c(self.2), c(self.3)]
    }

    pub fn darken(c: Color, factor: f32) -> Color {
        let avg = ((c.0 as f32 + c.1 as f32 + c.2 as f32) / 3.0) as u8;
        let f = |ch: u8| {
            let dark = (ch as f32 * factor) as u8;
            let sat_boost = dark as i16 + ((dark as i16 - avg as i16) / 5);
            sat_boost.clamp(0, 255) as u8
        };
        Color(f(c.0), f(c.1), f(c.2), c.3)
    }

    pub fn is_high_contrast(c: Color) -> bool {
        0.299*(c.0 as f32) + 0.587*(c.1 as f32) + 0.114*(c.2 as f32) > 128.0
    }
}
