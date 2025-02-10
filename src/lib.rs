use wgpu::{
    DepthStencilState,
    MultisampleState,
    TextureFormat,
    RenderPass,
    Device,
    Queue,
};

use wgpu_cyat::{CyatRenderer, ShapeArea, DefaultAttributes};
use wgpu_image::{ImageRenderer, Image, image::RgbaImage, ImageAttributes, ImageAtlas, ImageKey};
use glyphat::{TextRenderer, TextArea, FontAtlas, FontKey};

mod shape;
use shape::{CanvasShapeBuilder};

pub use wgpu_image::image;
pub use shape::{DrawCommand, Shape};
pub use glyphat::{Text};

pub enum ItemType {
    Shape(Shape, &'static str, Option<u32>),//Shape, Color, Stroke Width
    Text(Text),//Text
    Image(Shape, ImageKey)//Shape, Image
}

impl ItemType {
    pub fn size(&self, atlas: &mut CanvasAtlas) -> (u32, u32) {
        match &self {
            ItemType::Text(text) => atlas.messure_text(text),
            ItemType::Shape(shape, _, _) | ItemType::Image(shape, _) => shape.size(),
        }
    }
}

pub struct CanvasItem(pub ItemType, pub u32, pub (u32, u32), pub (u32, u32, u32, u32));

impl CanvasItem {
    pub fn size(&self, ctx: &mut CanvasAtlas) -> (u32, u32) {
        self.0.size(ctx)
    }

    fn point(w: f32, h: f32, x: u32, y: u32) -> [f32; 2] {
        [-1.0+(x as f32 / w), 1.0-(y as f32 / h)]
    }

    fn build(
        mut self, width: f32, height: f32, scale_factor: f64
    ) -> (Option<TextArea>, (Option<ShapeArea>, Option<Image>)) {
        let s = |u: u32| (u as f64 * scale_factor) as u32;
        self.3 = (s(self.3.0), s(self.3.1), s(self.3.2), s(self.3.3));
        match self.0 {
            ItemType::Text(text) => {
                (
                    Some(TextArea::new(text, self.1, self.2, self.3)),
                    (
                        None,
                        None
                    )
                )
            },
            ItemType::Shape(shape, color, stroke_width) => {
                let ce = "Color was not a Hex Value";
                let a = DefaultAttributes{
                    color: hex::decode(color).expect(ce).into_iter().map(|u| {
                        u as f32 / 255.0
                    }).collect::<Vec<f32>>().try_into().expect(ce),
                    z: self.1 as f32 / u32::MAX as f32
                };
                (
                    None,
                    (
                        Some(ShapeArea(CanvasShapeBuilder::new(
                            shape, stroke_width, a, self.2, width, height
                        ).into(), self.3)),
                        None
                    )
                )
            },
            ItemType::Image(shape, key) => {
                let size = shape.size();
                let a = ImageAttributes{
                    start: Self::point(width, height, self.2.0, self.2.1),
                    end: Self::point(width, height, self.2.0+size.0, self.2.1+size.1),
                    z: self.1 as f32 / u32::MAX as f32
                };
                (
                    None,
                    (
                        None,
                        Some(Image(CanvasShapeBuilder::new(
                            shape, None, a, self.2, width, height
                        ).into(), self.3, key))
                    )
                )
            }
        }
    }
}

pub struct CanvasAtlas {
    image: ImageAtlas,
    font: FontAtlas,
}

impl Default for CanvasAtlas {
    fn default() -> Self {
        Self::new()
    }
}

impl CanvasAtlas {
    pub fn new() -> Self {
        CanvasAtlas{
            image: ImageAtlas::new(),
            font: FontAtlas::new()
        }
    }

    pub fn add_image(&mut self, image: RgbaImage) -> ImageKey {self.image.add(image)}
    pub fn remove_image(&mut self, key: &ImageKey) {self.image.remove(key)}
    pub fn contains_image(&mut self, key: &ImageKey) -> bool {self.image.contains(key)}

    pub fn add_font(&mut self, font: Vec<u8>) -> FontKey {self.font.add(font)}
    pub fn remove_font(&mut self, key: &FontKey) {self.font.remove(key)}
    pub fn contains_font(&mut self, key: &FontKey) -> bool {self.font.contains(key)}

    pub fn messure_text(&mut self, t: &Text) -> (u32, u32) {self.font.messure_text(t)}
}

pub struct CanvasRenderer {
    cyat_renderer: CyatRenderer,
    text_renderer: TextRenderer,
    image_renderer: ImageRenderer,
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
            cyat_renderer: CyatRenderer::new(device, texture_format, multisample, depth_stencil.clone()),
            text_renderer: TextRenderer::new(queue, device, texture_format, multisample, depth_stencil.clone()),
            image_renderer: ImageRenderer::new(device, texture_format, multisample, depth_stencil)
        }
    }

    /// Prepare for rendering this frame; create all resources that will be
    /// used during the next render that do not already exist.
    #[allow(clippy::too_many_arguments)]
    pub fn prepare(
        &mut self,
        device: &Device,
        queue: &Queue,
        physical_width: u32,
        physical_height: u32,
        scale_factor: f64,
        logical_width: f32,
        logical_height: f32,
        canvas_atlas: &mut CanvasAtlas,
        items: Vec<CanvasItem>,
    ) {
        if items.is_empty() {return;}

        let (text_areas, shapes_images): (Vec<_>, Vec<_>) = items.into_iter().map(|m|
            m.build(logical_width / 2.0, logical_height / 2.0, scale_factor)
        ).unzip();
        let (shapes, images): (Vec<_>, Vec<_>) = shapes_images.into_iter().unzip();

        let text_areas: Vec<_> = text_areas.into_iter().flatten().collect();
        let shapes: Vec<_> = shapes.into_iter().flatten().collect();
        let images: Vec<_> = images.into_iter().flatten().collect();


        self.cyat_renderer.prepare(device, queue, shapes);

        self.text_renderer.prepare(
            queue,
            device,
            physical_width,
            physical_height,
            &mut canvas_atlas.font,
            text_areas
        );

        self.image_renderer.prepare(
            queue,
            device,
            &mut canvas_atlas.image,
            images
        );
    }

    /// Render using caller provided render pass.
    pub fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        self.cyat_renderer.render(render_pass);
        self.text_renderer.render(render_pass);
        self.image_renderer.render(render_pass);
    }
}
