use wgpu::{PipelineCompilationOptions, RenderPipelineDescriptor, PipelineLayoutDescriptor, DepthStencilState, MultisampleState, RenderPipeline, PrimitiveState, FragmentState, TextureFormat, BufferUsages, IndexFormat, VertexState, RenderPass, Device, Queue, VertexFormat, ShaderModule};
use wgpu_dyn_buffer::{DynamicBufferDescriptor, DynamicBuffer};

use crate::shape::{ShapeType, Shape, Vertex, Ellipse, Rectangle, RoundedRectangle};
use super::Area;

#[derive(Clone, Copy, Debug)]
pub struct Color(pub u8, pub u8, pub u8, pub u8);

impl Color {
    pub fn color(&self) -> [f32; 4] {
        let c = |f: u8| (((f as f32 / u8::MAX as f32) + 0.055) / 1.055).powf(2.4);
        [c(self.0), c(self.1), c(self.2), c(self.3)]
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ColorShape(pub Color, pub ShapeType);

#[derive(Clone, Copy, Debug)]
struct InnerColorShape<S: Shape>(pub Color, pub S);

impl<S: Shape> Shape for InnerColorShape<S> {
    type Vertex = ColorShapeVertex<S::Vertex>;

    fn build(self, width: u32, height: u32, a: Area, l: u16) -> ([Self::Vertex; 4], [u16; 6]) {
        let (v, i) = self.1.build(width, height, a, l);
        (v.into_iter().map(|vertex|
            ColorShapeVertex{vertex, color: self.0.color()}
        ).collect::<Vec<_>>().try_into().unwrap(), i)
    }
}

#[repr(C, packed)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ColorShapeVertex<V> {
    vertex: V,
    color: [f32; 4]
}

impl<V: Vertex> Vertex for ColorShapeVertex<V> {
    fn attributes() -> Vec<VertexFormat> {
        [V::attributes(), vec![VertexFormat::Float32x4]].concat()
    }
}

pub(crate) struct ColorShapeRenderer {
    ellipse_renderer: GenericColorRenderer<InnerColorShape<Ellipse>>,
    rectangle_renderer: GenericColorRenderer<InnerColorShape<Rectangle>>,
    rounded_rectangle_renderer: GenericColorRenderer<InnerColorShape<RoundedRectangle>>,
}

impl ColorShapeRenderer {
    /// Create all unchanging resources here.
    pub fn new(
        device: &Device,
        texture_format: &TextureFormat,
        multisample: MultisampleState,
        depth_stencil: Option<DepthStencilState>,
    ) -> Self {
        let ellipse_shader = device.create_shader_module(wgpu::include_wgsl!("color/ellipse.wgsl"));
        let rectangle_shader = device.create_shader_module(wgpu::include_wgsl!("color/rectangle.wgsl"));
        let rounded_rectangle_shader = device.create_shader_module(wgpu::include_wgsl!("color/rounded_rectangle.wgsl"));
        ColorShapeRenderer{
            ellipse_renderer: GenericColorRenderer::new(device, texture_format, multisample, depth_stencil.clone(), ellipse_shader),
            rectangle_renderer: GenericColorRenderer::new(device, texture_format, multisample, depth_stencil.clone(), rectangle_shader),
            rounded_rectangle_renderer: GenericColorRenderer::new(device, texture_format, multisample, depth_stencil.clone(), rounded_rectangle_shader),
        }
    }

    /// Prepare for rendering this frame; create all resources that will be
    /// used during the next render that do not already exist.
    pub fn prepare(
        &mut self,
        device: &Device,
        queue: &Queue,
        width: u32,
        height: u32,
        cs_area: Vec<(ColorShape, Area)>,
    ) {
        let (el_areas, rect_areas, rrect_areas) = cs_area.into_iter().fold(
            (vec![], vec![], vec![]), |mut a, (cs, area)| {
                match cs.1 {
                    ShapeType::Ellipse(ellipse) =>
                        a.0.push((InnerColorShape(cs.0, ellipse), area)),
                    ShapeType::Rectangle(rectangle) =>
                        a.1.push((InnerColorShape(cs.0, rectangle), area)),
                    ShapeType::RoundedRectangle(rounded_rectangle) =>
                        a.2.push((InnerColorShape(cs.0, rounded_rectangle), area))
                }
                a
            }
        );

        self.ellipse_renderer.prepare(device, queue, width, height, el_areas);
        self.rectangle_renderer.prepare(device, queue, width, height, rect_areas);
        self.rounded_rectangle_renderer.prepare(device, queue, width, height, rrect_areas);
    }

    /// Render using caller provided render pass.
    pub fn render(&self, render_pass: &mut RenderPass<'_>) {
        //Rectangle must go first to prevent alphamaping to map to the background behind the rectangle
        self.rectangle_renderer.render(render_pass);
        self.ellipse_renderer.render(render_pass);
        self.rounded_rectangle_renderer.render(render_pass);
    }
}

struct GenericColorRenderer<S: Shape> {
    render_pipeline: RenderPipeline,
    vertex_buffer: DynamicBuffer,
    index_buffer: DynamicBuffer,
    indices: u32,
    _marker: std::marker::PhantomData<S>
}

impl<S: Shape> GenericColorRenderer<S> {
    /// Create all unchanging resources here.
    pub fn new(
        device: &Device,
        texture_format: &TextureFormat,
        multisample: MultisampleState,
        depth_stencil: Option<DepthStencilState>,
        shader: ShaderModule,
    ) -> Self {
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor::default());
        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[S::Vertex::layout()]
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState{
                    format: *texture_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })]
            }),
            primitive: PrimitiveState::default(),
            depth_stencil,
            multisample,
            multiview: None,
            cache: None
        });

        let vertex_buffer = DynamicBuffer::new(device, &DynamicBufferDescriptor {
            label: None,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });

        let index_buffer = DynamicBuffer::new(device, &DynamicBufferDescriptor {
            label: None,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
        });

        GenericColorRenderer{
            render_pipeline,
            vertex_buffer,
            index_buffer,
            indices: 0,
            _marker: std::marker::PhantomData::<S>
        }
    }

    /// Prepare for rendering this frame; create all resources that will be
    /// used during the next render that do not already exist.
    pub fn prepare(
        &mut self,
        device: &Device,
        queue: &Queue,
        width: u32,
        height: u32,
        sh_area: Vec<(S, Area)>
    ) {
        let (vertices, indices) = sh_area.into_iter().fold((vec![], vec![]), |mut a, (sh, area)| {
            let (v, i) = sh.build(width, height, area, a.0.len() as u16);
            a.0.extend(v);
            a.1.extend(i);
            a
        });

        self.indices = indices.len() as u32;
        self.vertex_buffer.write_buffer(device, queue, bytemuck::cast_slice(&vertices));
        self.index_buffer.write_buffer(device, queue, bytemuck::cast_slice(&indices));
    }

    /// Render using caller provided render pass.
    pub fn render(&self, render_pass: &mut RenderPass<'_>) {
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.as_ref().slice(..));
        render_pass.set_index_buffer(self.index_buffer.as_ref().slice(..), IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.indices, 0, 0..1);
    }
}
