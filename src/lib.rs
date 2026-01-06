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
pub use text::{Font, Text, Span, Align};
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

#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    pub shape: ShapeType,
    pub image: Arc<RgbaImage>,
    pub color: Option<Color>
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
