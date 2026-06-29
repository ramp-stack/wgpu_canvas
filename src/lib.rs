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
impl Color {
    pub const WHITE: Self = Color(255, 255, 255, 255);
    pub const BLACK: Self = Color(0, 0, 0, 255);
    pub const TRANSPARENT: Self = Color(0, 0, 0, 0);
    pub const RED: Self = Color(255, 0, 0, 255);
    pub const GREEN: Self = Color(0, 255, 0, 255);
    pub const BLUE: Self = Color(0, 0, 255, 255);
    pub const YELLOW: Self = Color(255, 255, 0, 255);
    pub const MAGENTA: Self = Color(255, 0, 255, 255);
    pub const CYAN: Self = Color(0, 255, 255, 255);
}

#[derive(Clone, PartialEq)]
pub struct Image {
    pub shape: ShapeType,
    pub image: Arc<RgbaImage>,
    pub color: Option<Color>
}
impl Image {
    pub fn scale(&mut self, scale: f32) {self.shape = self.shape.scale(scale);}
    pub fn size(&self) -> (f32, f32) {self.shape.size()}
}
impl std::fmt::Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Image").field("shape", &self.shape).field("image", &"Arc<RgbaImage>").field("color", &self.color).finish()
    }
}

#[derive(Clone, Debug, Copy, PartialEq)]
pub struct Shape {
    pub shape: ShapeType,
    pub color: Color
}
impl Shape {
    pub fn scale(&mut self, scale: f32) {self.shape = self.shape.scale(scale);}
    pub fn size(&self) -> (f32, f32) {self.shape.size()}
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Area {
    pub offset: (f32, f32),
    pub bounds: Option<(f32, f32, f32, f32)>,
}
impl Area {
    pub fn scale(self, scale: f32) -> Self {Area {
        offset: (scale*self.offset.0, scale*self.offset.1),
        bounds: self.bounds.map(|b| (scale*b.0, scale*b.1, scale*b.2, scale*b.3)),
    }}
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Shape(Shape),
    Image(Image),
    Text(Text),
}
impl Item {
    pub fn scale(&mut self, scale: f32) {match self {
        Item::Shape(shape) => shape.scale(scale),
        Item::Image(image) => image.scale(scale),
        Item::Text(text) => text.scale(scale),
    }}

    pub fn size(&self) -> (f32, f32) {match self {
        Item::Shape(shape) => shape.size(),
        Item::Image(image) => image.size(),
        Item::Text(text) => text.size(),
    }}
}

#[derive(Debug, Clone,  PartialEq)]
pub struct Instruction(pub Area, pub Item);
impl Instruction {
    pub fn scale(&mut self, scale: f32) {
        self.0 = self.0.scale(scale);
        self.1.scale(scale);
    }
}
