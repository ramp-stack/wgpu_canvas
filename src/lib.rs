use wgpu::{
    PipelineCompilationOptions,
    RenderPipelineDescriptor,
    PipelineLayoutDescriptor,
    COPY_BUFFER_ALIGNMENT,
    VertexBufferLayout,
    DepthStencilState,
    MultisampleState,
    BufferDescriptor,
    RenderPipeline,
    PrimitiveState,
    VertexStepMode,
    FragmentState,
    TextureFormat,
    BufferAddress,
    ShaderModule,
    BufferUsages,
    IndexFormat,
    VertexState,
    RenderPass,
    Buffer,
    Device,
    Queue,
};

use lyon_tessellation::{
    FillVertexConstructor,
    BuffersBuilder,
    VertexBuffers,
    FillTessellator,
    FillVertex,
    FillBuilder,
    FillOptions
};


use lyon_path::{
    builder::{BorderRadii, PathBuilder},
    Winding
};

use lyon_tessellation::math::{Vector, Angle, Point, Box2D};

use wgpu_lyon::LyonRenderer;

//How precise the circles are
const TOLERANCE: f32 = 0.0001;

pub enum Shape {
  //Text(&'static str, u32, String),//text, scale, font
    //RoundedRectangle(u32, u32, u32),
  //Triangle(Vec2, Vec2, Vec2),
    Rectangle(u32, u32),
    Circle(u32)
}

pub struct Mesh {
    pub shape: Shape,
    pub offset: (u32, u32),
    pub color: &'static str
}

impl Mesh {
    fn vector(w: f32, h: f32, r: u32) -> Vector {
        Vector::new(r as f32 / w, r as f32 / h)
    }
    fn point(w: f32, h: f32, x: u32, y: u32) -> Point {
        Point::new(-1.0+(x as f32 / w), 1.0-(y as f32 / h))
    }

    fn build(
        &self, builder: &mut FillBuilder,
        width: f32, height: f32
    ) {
        let color_error = "Color was not a Hex Value";
        let color: [f32; 3] = hex::decode(self.color).expect(color_error).into_iter().map(|u|
            u as f32 / 255.0
        ).collect::<Vec<f32>>().try_into().expect(color_error);
        match self.shape {
          //Shape::Text(text, s, font) => {
          //    let p = lr(s);
          //    let text = Text::new(text)
          //        .with_scale(wgpu_glyph::ab_glyph::PxScale::from(48.0))
          //        .with_color([self.color.r as f32, self.color.g as f32, self.color.b as f32, 1.0]);
          //    return vec![text];
          //},
          //Shape::RoundedRectangle(w, h, r) => {
          //    builder.add_rounded_rectangle(
          //        &Box2D::new(
          //            Point::new(Self::px(lw, self.offset.0), Self::py(lh, self.offset.1)),
          //            Point::new(Self::px(lw, self.offset.0+w), Self::py(lh, self.offset.1+h)),
          //        ),
          //        &BorderRadii::new(0.1),
          //        Winding::Positive,
          //        &color
          //    )
          //},
            Shape::Rectangle(w, h) => {
                builder.add_rectangle(
                    &Box2D::new(
                        Self::point(width, height, self.offset.0, self.offset.1),
                        Self::point(width, height, self.offset.0+w, self.offset.1+h),
                    ),
                    Winding::Positive,
                    &color
                );
            },
            Shape::Circle(r) => {
                builder.add_ellipse(
                    Self::point(width, height, self.offset.0+r, self.offset.1+r),
                    Self::vector(width, height, r),
                    Angle::radians(0.0),
                    Winding::Positive,
                    &color
                )
            }
        }
    }
}

pub struct CanvasRenderer {
    lyon_renderer: LyonRenderer,
}

impl CanvasRenderer {
    /// Create all unchanging resources here.
    pub fn new(
        device: &Device,
        texture_format: &TextureFormat,
        multisample: MultisampleState,
        depth_stencil: Option<DepthStencilState>,
    ) -> Self {
        CanvasRenderer{
            lyon_renderer: LyonRenderer::new(device, texture_format, multisample, depth_stencil),
        }
    }

    /// Prepare for rendering this frame; create all resources that will be
    /// used during the next render that do not already exist.
    pub fn prepare(
        &mut self,
        device: &Device,
        queue: &Queue,
        meshes: Vec<Mesh>,
        width: f32,
        height: f32,
    ) {
        if meshes.is_empty() {return;}

        let callbacks = meshes.into_iter().map(|m|
            move |builder: &mut FillBuilder| {m.build(builder, width, height)}
        ).collect::<Vec<_>>();

        let mut fill_options = FillOptions::default();
        fill_options.tolerance = TOLERANCE;

        self.lyon_renderer.prepare(device, queue, &fill_options, callbacks);
    }

    /// Render using caller provided render pass.
    pub fn render(&self, render_pass: &mut RenderPass<'_>) {
        self.lyon_renderer.render(render_pass);
    }
}
