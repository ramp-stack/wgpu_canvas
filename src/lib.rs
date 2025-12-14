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

pub use text::{Font, Text, Span, Align, Cursor};
pub use image::RgbaImage;

#[derive(Default, Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Color(pub u8, pub u8, pub u8, pub u8);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shape {
    shape: ShapeType,
    color: Color
}

#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    shape: ShapeType,
    image: Arc<RgbaImage>,
    color: Option<Color>
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Shape(Shape),
    Image(Image),
    Text(Text),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShapeType {
    Ellipse(f32, (f32, f32), f32),
    Rectangle(f32, (f32, f32), f32),
    RoundedRectangle(f32, (f32, f32), f32, f32),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Area {
    offset: (f32, f32),
    bounds: Option<(f32, f32, f32, f32)>
}
