use wgpu::{DepthStencilState, MultisampleState, TextureFormat, RenderPass, Device, Queue};

mod shape;
mod color;
mod image;
mod text;

use color::ColorRenderer;
use image::{ImageRenderer, ImageAtlas};
use text::{TextRenderer, FontAtlas};

#[derive(Debug, Clone, Copy)]
pub struct Area {
    pub z_index: u16,//area.z_index = u16::MAX-area.z_index;
    pub offset: (u32, u32),
    pub bounds: (u32, u32, u32, u32)
}

#[derive(Clone, Debug)]
pub struct Text {
    pub text: &'static str,
    pub color: (u8, u8, u8, u8),
    pub width: Option<u32>,
    pub size: u32,
    pub line_height: u32,
    pub font: Font,
}

impl Text {
    pub fn new(
        text: &'static str,
        color: (u8, u8, u8, u8),
        width: Option<u32>,
        size: u32,
        line_height: u32,
        font: Font,
    ) -> Self {
        Text{text, color, width, size, line_height, font}
    }

    fn into_inner(self) -> text::Text {
        text::Text{text: self.text, color: self.color, width: self.width, size: self.size, line_height: self.line_height, font: self.font.into_inner()}
    }
}

#[derive(Clone, Debug, Copy)]
pub enum Shape {
    Ellipse(u32, (u32, u32)),
    Rectangle(u32, (u32, u32)),
    RoundedRectangle(u32, (u32, u32), u32),
}

#[derive(Clone, Debug)]
pub struct Font(text::Font);

impl Font {
    pub fn new(atlas: &mut CanvasAtlas, bytes: Vec<u8>) -> Self {
        Font(atlas.font.add(bytes))
    }

    fn into_inner(self) -> text::Font {
        self.0
    }
}

#[derive(Clone, Debug)]
pub struct Image(image::Image);

impl Image {
    pub fn new(atlas: &mut CanvasAtlas, image: image::RgbaImage) -> Self {
        Image(atlas.image.add(image))
    }

    fn into_inner(self) -> image::Image {
        self.0
    }
}

#[derive(Clone, Debug)]
pub enum CanvasItem {
    Shape(Shape, (u8, u8, u8, u8)),
    Image(Shape, Image),
    Text(Text),
}

#[derive(Default)]
pub struct CanvasAtlas {
    image: ImageAtlas,
    font: FontAtlas,
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
    #[allow(clippy::too_many_arguments)]
    pub fn prepare(
        &mut self,
        device: &Device,
        queue: &Queue,
        width: u32,
        height: u32,
        atlas: &mut CanvasAtlas,
        items: Vec<(Area, CanvasItem)>,
    ) {
        let (colors, images, texts) = items.into_iter().fold((vec![], vec![], vec![]), |mut a, (area, item)| {
            match item {
                CanvasItem::Shape(shape, color) => a.0.push((shape, color, area)),
                CanvasItem::Image(shape, image) => a.1.push((shape, image.into_inner(), area)),
                CanvasItem::Text(text) => a.2.push((text.into_inner(), area)),
            }
            a
        });

        self.color_renderer.prepare(device, queue, width, height, colors);
        self.image_renderer.prepare(device, queue, width, height, &mut atlas.image, images);
        self.text_renderer.prepare(device, queue, width, height, &mut atlas.font, texts);
    }

    /// Render using caller provided render pass.
    pub fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        self.color_renderer.render(render_pass);
        self.image_renderer.render(render_pass);
        self.text_renderer.render(render_pass);
    }
}
