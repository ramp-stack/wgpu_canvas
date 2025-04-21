use wgpu::{DepthStencilState, MultisampleState, TextureFormat, RenderPass, Device, Queue};

mod shape;
mod color;
mod image;
mod text;

use color::ColorRenderer;
use image::ImageRenderer;
use text::TextRenderer;

pub use color::Color;
pub use image::{ImageAtlas, Image};
pub use text::{FontAtlas, Font, Text};

#[derive(Debug, Clone, Copy)]
pub struct Area(pub (f32, f32), pub Option<(f32, f32, f32, f32)>);

impl Area {
    pub(crate) fn bounds(&self, width: f32, height: f32) -> (f32, f32, f32, f32) {
        self.1.unwrap_or((0.0, 0.0, width, height))
    }
}

#[derive(Clone, Debug, Copy)]
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

#[derive(Clone, Debug)]
pub enum CanvasItem {
    Shape(Shape, Color),
    Image(Shape, Image, Option<Color>),
    Text(Text),
}

pub struct CanvasRenderer {
    color_renderer: ColorRenderer,
    image_renderer: ImageRenderer,
    text_renderer: TextRenderer,
}

impl CanvasRenderer {
    /// Create all unchanging resources here.
    pub fn new(
        queue: &Queue,
        device: &Device,
        texture_format: &TextureFormat,
        multisample: MultisampleState,
        depth_stencil: Option<DepthStencilState>,
    ) -> Self {
        CanvasRenderer{
            color_renderer: ColorRenderer::new(device, texture_format, multisample, depth_stencil.clone()),
            image_renderer: ImageRenderer::new(device, texture_format, multisample, depth_stencil.clone()),
            text_renderer: TextRenderer::new(device, queue, texture_format, multisample, depth_stencil),
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
        image_atlas: &mut ImageAtlas,
        font_atlas: &mut FontAtlas,
        items: Vec<(Area, CanvasItem)>,
    ) {
        let (colors, images, texts) = items.into_iter().enumerate().fold((vec![], vec![], vec![]), |mut a, (i, (area, item))| {
            let z = i as u16;
            match item {
                CanvasItem::Shape(shape, color) => a.0.push((z, area, shape, color)),
                CanvasItem::Image(shape, image, color) => a.1.push((z, area, shape, image, color)),
                CanvasItem::Text(text) => a.2.push((z, area, text)),
            }
            a
        });

        self.color_renderer.prepare(device, queue, width, height, colors);
        self.image_renderer.prepare(device, queue, width, height, image_atlas, images);
        self.text_renderer.prepare(device, queue, width, height, font_atlas, texts);
    }

    /// Render using caller provided render pass.
    pub fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        self.color_renderer.render(render_pass);
        self.image_renderer.render(render_pass);
        self.text_renderer.render(render_pass);
    }
}
