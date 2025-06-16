use wgpu::{DepthStencilState, MultisampleState, TextureFormat, RenderPass, Device, Queue};

mod shape;
mod color;
mod image;
mod text;
//mod cursor;

use color::ColorRenderer;
use image::ImageRenderer;

pub use color::Color;
pub use image::{ImageAtlas, Image};
pub use text::{TextAtlas, Font, Text, Span, Align, Cursor};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Area(pub (f32, f32), pub Option<(f32, f32, f32, f32)>);

impl Area {
    pub(crate) fn bounds(&self, width: f32, height: f32) -> (f32, f32, f32, f32) {
        self.1.unwrap_or((0.0, 0.0, width, height))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shape {
    Ellipse(f32, (f32, f32)),
    Rectangle(f32, (f32, f32)),
    RoundedRectangle(f32, (f32, f32), f32),
}

impl Shape {
    pub fn size(&self) -> (f32, f32) {
        match self {
            Shape::Ellipse(_, size) => *size,
            Shape::Rectangle(_, size) => *size,
            Shape::RoundedRectangle(_, size, _) => *size,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Shape(Shape, Color),
    Image(Shape, Image, Option<Color>),
    Text(Text),
}

#[derive(Default)]
pub struct Atlas {
    pub(crate) image: ImageAtlas,
    pub(crate) text: TextAtlas,
}

impl Atlas {
    pub fn add_image(&mut self, raw: crate::image::RgbaImage) -> Image {self.image.add(raw)}
    pub fn add_font(&mut self, raw_font: &[u8]) -> Result<Font, &'static str> {self.text.add(raw_font)}
}

impl AsMut<TextAtlas> for Atlas {fn as_mut(&mut self) -> &mut TextAtlas {&mut self.text}}
impl AsMut<ImageAtlas> for Atlas {fn as_mut(&mut self) -> &mut ImageAtlas {&mut self.image}}

pub struct Renderer {
    color_renderer: ColorRenderer,
    image_renderer: ImageRenderer,
}

impl Renderer {
    /// Create all unchanging resources here.
    pub fn new(
        device: &Device,
        texture_format: &TextureFormat,
        multisample: MultisampleState,
        depth_stencil: Option<DepthStencilState>,
    ) -> Self {
        Renderer{
            color_renderer: ColorRenderer::new(device, texture_format, multisample, depth_stencil.clone()),
            image_renderer: ImageRenderer::new(device, texture_format, multisample, depth_stencil.clone()),
        }
    }

    /// Prepare for rendering this frame; create all resources that will be
    /// used during the next render that do not already exist.
    ///
    /// Items are given a z_index based on the order in which they are presented. First item in the
    /// vector will be printed in the back of the stack(z = u16::MAX-1)
    #[allow(clippy::too_many_arguments)]
    pub fn prepare(
        &mut self,
        device: &Device,
        queue: &Queue,
        width: f32,
        height: f32,
        atlas: &mut Atlas,
        items: Vec<(Area, Item)>,
    ) {
        let (colors, mut images, texts) = items.into_iter().enumerate().fold((vec![], vec![], vec![]), |mut a, (i, (area, item))| {
            let z = i as u16;
            match item {
                Item::Shape(shape, color) => a.0.push((z, area, shape, color)),
                Item::Image(shape, image, color) => a.1.push((z, area, shape, image, color)),
                Item::Text(text) => a.2.push((z, area, text)),
            }
            a
        });

        images.extend(atlas.text.prepare_images(&mut atlas.image, texts));

        self.color_renderer.prepare(device, queue, width, height, colors);
        self.image_renderer.prepare(device, queue, width, height, &mut atlas.image, images);
    }

    /// Render using caller provided render pass.
    pub fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        self.color_renderer.render(render_pass);
        self.image_renderer.render(render_pass);
    }
}
