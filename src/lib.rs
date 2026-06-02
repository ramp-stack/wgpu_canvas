use std::sync::Arc;

#[cfg(feature = "renderer")]
mod renderer;
#[cfg(feature = "renderer")]
pub use renderer::{Renderer, Atlas};

#[cfg(feature = "canvas")]
mod canvas;
#[cfg(feature = "canvas")]
pub use canvas::Canvas;

mod text;
pub use text::{Font, Text, Span, Align, Character};
//TODO: replace shape enum with a single definition with optional corner radius
//Squash rectangles into rounded rectangles. Ignore corner radius on Ellipse
mod shape;
pub use shape::Shape as ShapeType;

pub use image::RgbaImage;

#[derive(Default, Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Color(pub u8, pub u8, pub u8, pub u8);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shape {
    pub shape: ShapeType,
    pub color: Color
}

#[derive(Clone, PartialEq)]
pub struct Image {
    pub shape: ShapeType,
    pub image: Arc<RgbaImage>,
    pub color: Option<Color>
}

impl std::fmt::Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Image").field("shape", &self.shape).field("image", &"Arc<RgbaImage>").field("color", &self.color).finish()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Shape(Shape),
    Image(Image),
    Text(Text),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Area {
    pub offset: (f32, f32),
    pub bounds: Option<(f32, f32, f32, f32)>
}


// text.rs

// Character gained a 7th field pub f32 for font_size
// #[derive(Debug, Clone)] added back to Character (accidentally dropped)
// Every Character(...) construction passes s.font_size as the 7th argument

// atlas.rs

// ImageMap key changed from (char, u32) to (char, u32, u32) to include scale factor bits
// get_image takes font_size: f32, scale_factor: f32 and rasterizes at font_size * scale_factor instead of hardcoded 160.0
// get takes scale_factor: f32 and passes it to get_image, reads ch.6 for font size
// clear() method added to TextAtlas
// get_font return type updated to &mut ImageMap

// image.rs

// text_sampler added as a pub field using FilterMode::Nearest for pixel-exact glyph sampling

// renderer.rs

// prepare takes scale_factor: f32
// atlas.text.get(text) changed to atlas.text.get(text, scale_factor)
// Glyph offsets are rounded with .round() to avoid sub-pixel blurring

// canvas.rs

// scale_factor: f32 field added, hardcoded to 2.0
// set_scale_factor() method added — clears text atlas and resets old items when scale changes
// draw() passes self.scale_factor to renderer.prepare