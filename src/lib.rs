use wgpu::{DepthStencilState, MultisampleState, TextureFormat, RenderPass, Device, Queue};

mod shape;
mod image;
mod text;

pub use shape::{ShapeKey, Shape, Ellipse};
pub use image::{ImageKey, Image};
pub use text::{FontKey, Text};

use shape::{ShapeAtlas};
use image::{ImageRenderer, ImageAtlas};
use text::{TextRenderer, FontAtlas};

#[derive(Debug, Clone, Copy)]
pub struct Area {
    pub z_index: u16,
    pub offset: (u32, u32),
    pub bounds: (u32, u32, u32, u32)
}

#[derive(Debug, Clone, Copy)]
pub enum ItemType {
    Shape(ShapeKey),
    Image(ImageKey),
    Text(Text),
}

#[derive(Debug, Clone, Copy)]
pub struct CanvasItem {
    pub area: Area,
    pub item_type: ItemType,
}

#[derive(Default)]
pub struct CanvasAtlas {
    shape: ShapeAtlas,
    image: ImageAtlas,
    font: FontAtlas,
}

impl CanvasAtlas {
    pub fn add_shape(&mut self, shape: impl Shape) -> ShapeKey {self.shape.add(shape, &mut self.image)}
    pub fn remove_shape(&mut self, key: &ShapeKey) {self.shape.remove(key)}

    pub fn add_image(&mut self, image: Image) -> ImageKey {self.image.add(image)}
    pub fn remove_image(&mut self, key: &ImageKey) {self.image.remove(key)}

    pub fn add_font(&mut self, font: Vec<u8>) -> FontKey {self.font.add(font)}
    pub fn remove_font(&mut self, key: &FontKey) {self.font.remove(key)}

    pub fn messure_text(&mut self, t: &Text) -> (u32, u32) {self.font.messure_text(t)}
}

pub struct CanvasRenderer {
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
            image_renderer: ImageRenderer::new(device, texture_format, multisample, depth_stencil.clone()),
            text_renderer: TextRenderer::new(device, queue, texture_format, multisample, depth_stencil.clone()),
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
        items: Vec<CanvasItem>
    ) {
        let (images, texts) = items.into_iter().fold(
            (vec![], vec![]), |mut a, item| {
                let mut area = item.area;
                area.z_index = u16::MAX-area.z_index;
                match item.item_type {
                    ItemType::Shape(shape) => a.0.push((atlas.shape.get(&shape), area)),
                    ItemType::Image(image) => a.0.push((image, area)),
                    ItemType::Text(text) => a.1.push((text, area)),
                };
                a
            }
        );

        self.image_renderer.prepare(device, queue, width, height, &mut atlas.image, images);
        self.text_renderer.prepare(device, queue, width, height, &mut atlas.font, texts);
    }

    /// Render using caller provided render pass.
    pub fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        self.image_renderer.render(render_pass);
        self.text_renderer.render(render_pass);
    }
}
