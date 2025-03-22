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
    pub z_index: u16,
    pub offset: (i32, i32),
    pub bounds: (i32, i32, u32, u32)
}

#[derive(Clone, Copy, Debug)]
pub struct Color(pub u8, pub u8, pub u8, pub u8);

impl Color {
    pub(crate) fn color(&self) -> [f32; 4] {
        let c = |f: u8| (((f as f32 / u8::MAX as f32) + 0.055) / 1.055).powf(2.4);
        [c(self.0), c(self.1), c(self.2), c(self.3)]
    }
}

#[derive(Clone, Debug)]
pub struct Text {
    pub text: String,
    pub color: Color,
    pub width: Option<u32>,
    pub size: u32,
    pub line_height: u32,
    pub font: Font,
}

impl Text {
    pub fn size(&self, atlas: &mut CanvasAtlas) -> (u32, u32) {
        atlas.font.messure_text(&self.clone().into_inner())
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

impl Shape {
    pub fn size(&self) -> (u32, u32) {
        match self {
            Shape::Ellipse(_, size) => *size,
            Shape::Rectangle(_, size) => *size,
            Shape::RoundedRectangle(_, size, _) => *size,
        }
    }
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
    Shape(Shape, Color),
    Image(Shape, Image, Option<Color>),
    Text(Text),
}

impl CanvasItem {
    pub fn size(&self, atlas: &mut CanvasAtlas) -> (u32, u32) {
        match self {
            CanvasItem::Shape(shape, _) => shape.size(),
            CanvasItem::Image(shape, _, _) => shape.size(),
            CanvasItem::Text(text) => text.size(atlas)
        }
    }
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
        let (colors, images, texts) = items.into_iter().fold((vec![], vec![], vec![]), |mut a, (mut area, item)| {
            area.z_index = u16::MAX-area.z_index;
            match item {
                CanvasItem::Shape(shape, color) => a.0.push((area, shape, color)),
                CanvasItem::Image(shape, image, color) => a.1.push((area, shape, image.into_inner(), color)),
                CanvasItem::Text(text) => a.2.push((area, text.into_inner())),
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
