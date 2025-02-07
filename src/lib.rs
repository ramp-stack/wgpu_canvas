use wgpu::{
    DepthStencilState,
    MultisampleState,
    TextureFormat,
    RenderPass,
    Device,
    Queue,
};

use lyon_tessellation::{
    FillBuilder,
    FillOptions
};
use lyon_path::{
    builder::PathBuilder,
    Winding
};
use lyon_tessellation::math::{Vector, Angle, Point, Box2D};
use wgpu_lyon::{LyonRenderer, Shape as LyonShape};
use wgpu_image::{ImageRenderer, Image, image::RgbaImage};

mod text;
use text::{TextRenderer, Text};

//How precise the circles are
const TOLERANCE: f32 = 0.0001;

type Callback = Box<dyn Fn(&mut FillBuilder) + 'static>;

pub enum Shape {
    Rectangle(u32, u32),
    Circle(u32)
}

impl Shape {
    pub fn size(&self) -> (u32, u32) {
        match self {
            Shape::Rectangle(w, h) => (*w, *h),
            Shape::Circle(r) => (r*2, r*2)
        }
    }

    fn vector(w: f32, h: f32, r: u32) -> Vector {
        Vector::new(r as f32 / w, r as f32 / h)
    }

    fn point(w: f32, h: f32, x: u32, y: u32) -> Point {
        Point::new(-1.0+(x as f32 / w), 1.0-(y as f32 / h))
    }

    pub fn build(self, width: f32, height: f32, offset: (u32, u32), attrs: &'static [f32]) -> Callback {
        match self {
            Shape::Rectangle(w, h) => {
                Box::new(move |builder: &mut FillBuilder| {
                    builder.add_rectangle(
                        &Box2D::new(
                            Self::point(width, height, offset.0, offset.1),
                            Self::point(width, height, offset.0+w, offset.1+h),
                        ),
                        Winding::Positive,
                        attrs
                    );
                })
            },
            Shape::Circle(r) => {
                Box::new(move |builder: &mut FillBuilder| {
                    builder.add_ellipse(
                        Self::point(width, height, offset.0+r, offset.1+r),
                        Self::vector(width, height, r),
                        Angle::radians(0.0),
                        Winding::Positive,
                        attrs
                    );
                })
            }
        }
    }
}

pub enum MeshType<'a> {
    Shape(Shape, &'a str),//Shape, Color
    Text(&'a str, Option<u32>, &'a str, &'a [u8], f32, f32),//color, width_opt, text, font, size, line_height
    Image(Shape, RgbaImage)//Shape, Image
}

pub struct Context<'a> {
    text_renderer: &'a mut TextRenderer
}

impl<'a> Context<'a> {
    pub fn messure_text(&mut self, text: &Text) -> (u32, u32) {
        self.text_renderer.messure_text(text)
    }
}

pub struct Mesh<'a> {
    pub mesh_type: MeshType<'a>,
    pub offset: (u32, u32),
    pub z_index: u32,
    pub bound: (u32, u32, u32, u32)
}

impl<'a> Mesh<'a> {
    fn point(w: f32, h: f32, x: u32, y: u32) -> Point {
        Point::new(-1.0+(x as f32 / w), 1.0-(y as f32 / h))
    }

    pub fn size(&self, ctx: &mut Context) -> (u32, u32) {
        match &self.mesh_type {
            MeshType::Text(color, width_opt, text, font, size, line_height) => {
                ctx.messure_text(
                    &Text::new(0, 0, *width_opt, text, color, font, *size, *line_height, 0, (0, 0, 0, 0))
                )
            },
            MeshType::Shape(shape, _) | MeshType::Image(shape, _) => shape.size(),
        }
    }

    fn build(
        self, width: f32, height: f32
    ) -> (Option<Text<'a>>, (Option<LyonShape>, Option<Image>)) {
        match self.mesh_type {
            MeshType::Text(color, width_opt, text, font, size, line_height) => {
                (
                    Some(Text::new(self.offset.0, self.offset.1, width_opt, text, color, font, size, line_height, self.z_index, self.bound)),
                    (
                        None,
                        None
                    )
                )
            },
            MeshType::Shape(shape, color) => {
                let ce = "Color was not a Hex Value";
                let color: [f32; 3] = hex::decode(color).expect(ce).into_iter().map(|u| {
                    u as f32 / 255.0
                }).collect::<Vec<f32>>().try_into().expect(ce);
                let z = self.z_index as f32 / u32::MAX as f32;
                let attrs = vec![color[0], color[1], color[2], z].leak();
                (
                    None,
                    (
                        Some(LyonShape{
                            constructor: shape.build(width, height, self.offset, attrs),
                            bound: self.bound
                        }),
                        None
                    )
                )
            },
            MeshType::Image(shape, image) => {
                let offset = Self::point(width, height, self.offset.0, self.offset.1).to_array();
                let size = shape.size();
                let end = Self::point(width, height, self.offset.0+size.0, self.offset.1+size.1);
                let z = self.z_index as f32 / u32::MAX as f32;
                let attrs = vec![offset[0], offset[1], end.x, end.y, z].leak();
                (
                    None,
                    (
                        None,
                        Some(Image{
                            shape_constructor: shape.build(width, height, self.offset, attrs),
                            bound: self.bound,
                            image
                        })
                    )
                )
            }
        }
    }
}

pub struct CanvasRenderer {
    lyon_renderer: LyonRenderer,
    text_renderer: TextRenderer,
    image_renderer: ImageRenderer
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
            lyon_renderer: LyonRenderer::new(device, texture_format, multisample, depth_stencil.clone()),
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
        logical_width: f32,
        logical_height: f32,
        meshes: Vec<Mesh>,
    ) {
        if meshes.is_empty() {return;}

        let (text, shapes_images): (Vec<_>, Vec<_>) = meshes.into_iter().map(|m|
            m.build(logical_width / 2.0, logical_height / 2.0)
        ).unzip();
        let (shapes, images): (Vec<_>, Vec<_>) = shapes_images.into_iter().unzip();

        let text: Vec<_> = text.into_iter().flatten().collect();
        let shapes: Vec<_> = shapes.into_iter().flatten().collect();
        let images: Vec<_> = images.into_iter().flatten().collect();

        let mut fill_options = FillOptions::default();
        fill_options.tolerance = TOLERANCE;

        self.lyon_renderer.prepare(device, queue, &fill_options, shapes);

        self.text_renderer.prepare(
            device,
            queue,
            physical_width,
            physical_height,
            text
        );

        self.image_renderer.prepare(device, queue, &fill_options, images);
    }

    /// Render using caller provided render pass.
    pub fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        self.lyon_renderer.render(render_pass);
        self.text_renderer.render(render_pass);
        self.image_renderer.render(render_pass);
    }

    pub fn context(&mut self) -> Context {
        Context{
            text_renderer: &mut self.text_renderer
        }
    }
}
