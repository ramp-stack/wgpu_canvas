use wgpu::{DepthStencilState, MultisampleState, TextureFormat, RenderPass, Device, Queue};

mod buffer;
mod vertex;
mod color;
use color::ColorRenderer;
mod image;
use image::ImageRenderer;
mod atlas;
pub use atlas::Atlas;

use crate::{Area, Item};

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

    #[allow(clippy::too_many_arguments)]
    pub fn prepare(
        &mut self,
        device: &Device,
        queue: &Queue,
        width: f32,
        height: f32,
        scale_factor: f32,
        atlas: &mut Atlas,
        items: Vec<(Area, Item)>,
    ) {
        let (colors, images) = items.into_iter().enumerate().fold((vec![], vec![]), |mut a, (i, (area, item))| {
            let z = i as u16;
            match item {
                Item::Shape(shape) => a.0.push((z, area, shape.shape, shape.color)),
                Item::Image(image) => a.1.push((z, area, image.shape, image.image, image.color)),
                Item::Text(text) => a.1.extend(atlas.text.get(text, scale_factor).into_iter().map(|(offset, shape, image, color)| {
                    let area = Area{
                        offset: (
                            (area.offset.0 + offset.0).round(),
                            (area.offset.1 + offset.1).round(),
                        ),
                        bounds: area.bounds
                    };
                    (z, area, shape, image, Some(color))
                }))
            }
            a
        });

        self.color_renderer.prepare(device, queue, width, height, colors);
        self.image_renderer.prepare(device, queue, width, height, &mut atlas.image, images);
    }

    /// Render using caller provided render pass.
    pub fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        self.color_renderer.render(render_pass);
        self.image_renderer.render(render_pass);
    }
}