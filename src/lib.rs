use wgpu::{DepthStencilState, MultisampleState, TextureFormat, RenderPass, Device, Queue};

mod shape;
mod image;
mod text;

pub use shape::Shape;
pub use text::Text;

use image::{ImageRenderer, ImageAtlas, ImagePointer};
use text::{TextRenderer, FontAtlas, FontPointer};

#[derive(Debug, Clone, Copy)]
pub struct Area {
    pub z_index: u16,//area.z_index = u16::MAX-area.z_index;
    pub offset: (u32, u32),
    pub bounds: (u32, u32, u32, u32)
}

#[derive(Clone, Debug)]
pub struct Font(FontPointer);

impl Font {
    pub fn new(atlas: &mut CanvasAtlas, bytes: Vec<u8>) -> Self {
        Font(atlas.font.add(bytes))
    }
}

#[derive(Clone, Debug)]
pub struct Image(ImagePointer);

impl Image {
    pub fn new(atlas: &mut CanvasAtlas, image: image::RgbaImage) -> Self {
        Image(atlas.image.add(image))
    }
}

#[derive(Clone, Debug)]
pub enum CanvasItem {
    Shape(ImagePointer),
    Image(ImagePointer),
    Text(Text),
}

impl CanvasItem {
    pub fn text(
        text: &'static str,
        color: (u8, u8, u8, u8),
        width: Option<u32>,
        size: u32,
        line_height: u32,
        font: Font,
    ) -> Self {
        CanvasItem::Text(Text::new(text, color, width, size, line_height, font.0))
    }

    pub fn shape(atlas: &mut CanvasAtlas, shape: Shape, color: (u8, u8, u8, u8)) -> Self {
        CanvasItem::Shape(atlas.image.add(shape.color(color)))
    }

    pub fn image(atlas: &mut CanvasAtlas, shape: Shape, image: Image) -> Self {
        CanvasItem::Image(atlas.image.add(shape.image(image.0)))
    }

    pub fn size(&self, atlas: &mut CanvasAtlas) -> (u32, u32) {
        match self {
            CanvasItem::Shape(image) => image.dimensions(),
            CanvasItem::Image(image) => image.dimensions(),
            CanvasItem::Text(text) => atlas.font.messure_text(text),
        }
    }
}


#[derive(Default)]
pub struct CanvasAtlas {
    image: ImageAtlas,
    font: FontAtlas,
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
        let (images, texts) = items.into_iter().fold((vec![], vec![]), |mut a, (area, item)| {
            match item {
                CanvasItem::Shape(image) => a.0.push((image, area)),
                CanvasItem::Image(image) => a.0.push((image, area)),
                CanvasItem::Text(text) => a.1.push((text, area)),
            }
            a
        });

      //let mut images: Vec<(ImagePointer, Area)> = images.into_iter().map(|(i, a)| (i.0, a)).collect();
      //images.extend(shapes.into_iter().map(|(s, a)| (s.0, a)));
      //let texts = texts.into_iter().map(|(t, a)| (t.0, a)).collect();

        self.image_renderer.prepare(device, queue, width, height, &mut atlas.image, images);
        self.text_renderer.prepare(device, queue, width, height, &mut atlas.font, texts);
    }

    /// Render using caller provided render pass.
    pub fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        self.image_renderer.render(render_pass);
        self.text_renderer.render(render_pass);
    }
}
