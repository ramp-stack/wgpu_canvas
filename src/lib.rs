use wgpu::{DepthStencilState, MultisampleState, TextureFormat, RenderPass, Device, Queue};

pub mod shape;
pub mod color;
pub mod image;
pub mod text;

use color::{ColorShapeRenderer, ColorShape};
use image::{ImageShapeRenderer, ImageAtlas, ImageShape};
use text::{TextRenderer, FontAtlas, Text};

#[derive(Debug, Clone, Copy)]
pub struct Area {
    pub z_index: u16,
    pub offset: (u32, u32),
    pub bounds: (u32, u32, u32, u32)
}

#[derive(Debug, Clone, Copy)]
pub enum ItemType {
    ColorShape(ColorShape),
    ImageShape(ImageShape),
    Text(Text),
}

#[derive(Debug, Clone, Copy)]
pub struct CanvasItem {
    pub area: Area,
    pub item_type: ItemType,
}

#[derive(Default)]
pub struct CanvasAtlas {
    pub image: ImageAtlas,
    pub font: FontAtlas,
}

pub struct CanvasRenderer {
    color_shape_renderer: ColorShapeRenderer,
    image_shape_renderer: ImageShapeRenderer,
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
            color_shape_renderer: ColorShapeRenderer::new(device, texture_format, multisample, depth_stencil.clone()),
            image_shape_renderer: ImageShapeRenderer::new(device, texture_format, multisample, depth_stencil.clone()),
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
        let (color_shapes, image_shapes, texts) = items.into_iter().fold(
            (vec![], vec![], vec![]), |mut a, item| {
                let mut area = item.area;
                area.z_index = u16::MAX-area.z_index;
                match item.item_type {
                    ItemType::ColorShape(color_shape) => a.0.push((color_shape, area)),
                    ItemType::ImageShape(image_shape) => a.1.push((image_shape, area)),
                    ItemType::Text(text) => a.2.push((text, area)),
                };
                a
            }
        );

        self.color_shape_renderer.prepare(device, queue, width, height, color_shapes);
        self.image_shape_renderer.prepare(device, queue, width, height, &mut atlas.image, image_shapes);
        self.text_renderer.prepare(device, queue, width, height, &mut atlas.font, texts);
    }

    /// Render using caller provided render pass.
    pub fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        self.color_shape_renderer.render(render_pass);
        self.image_shape_renderer.render(render_pass);
        self.text_renderer.render(render_pass);
    }
}
