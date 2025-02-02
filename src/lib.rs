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
    builder::PathBuilder,
    Winding
};

use lyon_tessellation::math::{Box2D, Point};

use wgpu_lyon::{
    DefaultVertex,
    DefaultVertexConstructor,
    LyonRenderer
};

//How precise the circles are
const TOLERANCE: f32 = 0.0001;

pub enum Shape {
    //Triangle(Vec2, Vec2, Vec2),
    //Text(&'static str, u32, String),//text, scale, font
  //RoundedRectangle(u32, u32, u32),
    Rectangle(u32, u32),
  //Circle(u32)
}

pub struct Mesh {
    pub shape: Shape,
    pub offset: (u32, u32),
    //pub color: Color
}

impl Mesh {
    fn prepare(
        &self, builder: &mut FillBuilder
    ) {
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
          //            ctp(self.offset.0, self.offset.1),
          //            ctp(self.offset.0+w, self.offset.1+h)
          //        ),
          //        &BorderRadii::new(0.01),
          //        Winding::Positive,
          //        &[self.color.r as f32, self.color.g as f32, self.color.b as f32]
          //    )
          //},
            Shape::Rectangle(w, h) => {
                builder.add_rectangle(
                    &Box2D::new(
                        Point::new(0.1, 0.1),
                        Point::new(0.2, 0.2)
                    ),
                    Winding::Positive,
                    &[0.0, 0.0, 1.0]
                );
            },
          //Shape::Circle(r) => {
          //    builder.add_ellipse(
          //        ctp(self.offset.0+r, self.offset.1+r),
          //        lr(r),
          //        Angle::radians(0.0),
          //        Winding::Positive,
          //        &[self.color.r as f32, self.color.g as f32, self.color.b as f32]
          //    )
          //}
        }
    }
}


pub struct CanvasRenderer {
    lyon_renderer: LyonRenderer<DefaultVertex>,
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
        meshes: Vec<Mesh>
    ) {
        if meshes.is_empty() {return;}

        let callback = |builder: &mut FillBuilder| meshes.iter().for_each(|m| m.prepare(builder));

        let mut fill_options = FillOptions::default();
        fill_options.tolerance = TOLERANCE;

        self.lyon_renderer.prepare(device, queue, &fill_options, callback);
    }

    /// Render using caller provided render pass.
    pub fn render(&self, render_pass: &mut RenderPass<'_>) {
        self.lyon_renderer.render(render_pass);
    }
}
