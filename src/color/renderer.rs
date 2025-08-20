use wgpu::{PipelineCompilationOptions, RenderPipelineDescriptor, PipelineLayoutDescriptor, DepthStencilState, MultisampleState, RenderPipeline, PrimitiveState, FragmentState, TextureFormat, BufferUsages, IndexFormat, VertexState, RenderPass, Device, Queue, VertexBufferLayout, ShaderModule};
use wgpu_dyn_buffer::{DynamicBufferDescriptor, DynamicBuffer};

use crate::shape::{Vertex, ShapeVertex, RoundedRectangleVertex, ColorVertex};
use crate::{Area, Shape};
use super::Color;

pub struct ColorRenderer {
    ellipse_renderer: GenericColorRenderer,
    rectangle_renderer: GenericColorRenderer,
    rounded_rectangle_renderer: GenericColorRenderer,
}

impl ColorRenderer {
    /// Create all unchanging resources here.
    pub fn new(
        device: &Device,
        texture_format: &TextureFormat,
        multisample: MultisampleState,
        depth_stencil: Option<DepthStencilState>,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("ellipse.wgsl"));
        let ellipse_renderer = GenericColorRenderer::new(device, texture_format, multisample, depth_stencil.clone(), shader, ColorVertex::<ShapeVertex>::layout());
        let shader = device.create_shader_module(wgpu::include_wgsl!("rectangle.wgsl"));
        let rectangle_renderer = GenericColorRenderer::new(device, texture_format, multisample, depth_stencil.clone(), shader, ColorVertex::<ShapeVertex>::layout());
        let shader = device.create_shader_module(wgpu::include_wgsl!("rounded_rectangle.wgsl"));
        let rounded_rectangle_renderer = GenericColorRenderer::new(device, texture_format, multisample, depth_stencil.clone(), shader, ColorVertex::<RoundedRectangleVertex>::layout());
        ColorRenderer{
            ellipse_renderer,
            rectangle_renderer,
            rounded_rectangle_renderer
        }
    }

    /// Prepare for rendering this frame; create all resources that will be
    /// used during the next render that do not already exist.
    pub fn prepare(
        &mut self,
        device: &Device,
        queue: &Queue,
        width: f32,
        height: f32,
        items: Vec<(u16, Area, Shape, Color)>,
    ) {

        let (ellipses, rects, rounded_rects) = items.into_iter().fold(
            (vec![], vec![], vec![]),
            |mut a, (z, area, shape, color)| {
                match shape {
                    Shape::Ellipse(stroke, size, rotation) => a.0.push(ColorVertex::new(ShapeVertex::new(width, height, z, area, stroke, size, rotation), color)),
                    Shape::Rectangle(stroke, size, rotation) => a.1.push(ColorVertex::new(ShapeVertex::new(width, height, z, area, stroke, size, rotation), color)),
                    Shape::RoundedRectangle(stroke, size, corner_radius, rotation) =>
                        a.2.push(ColorVertex::new(RoundedRectangleVertex::new(width, height, z, area, stroke, size, corner_radius, rotation), color)),
                }
                a
            }
        );
        self.ellipse_renderer.prepare(device, queue, ellipses);
        self.rectangle_renderer.prepare(device, queue, rects);
        self.rounded_rectangle_renderer.prepare(device, queue, rounded_rects);
    }

    /// Render using caller provided render pass.
    pub fn render(&self, render_pass: &mut RenderPass<'_>) {
        self.ellipse_renderer.render(render_pass);
        self.rectangle_renderer.render(render_pass);
        self.rounded_rectangle_renderer.render(render_pass);
    }
}

pub struct GenericColorRenderer {
    render_pipeline: RenderPipeline,
    vertex_buffer: DynamicBuffer,
    index_buffer: DynamicBuffer,
    indices: u32
}

impl GenericColorRenderer {
    /// Create all unchanging resources here.
    pub fn new(
        device: &Device,
        texture_format: &TextureFormat,
        multisample: MultisampleState,
        depth_stencil: Option<DepthStencilState>,
        shader: ShaderModule,
        vertex_layout: VertexBufferLayout
    ) -> Self {
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor::default());

        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[vertex_layout]
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[
                    Some(wgpu::ColorTargetState{
                        format: *texture_format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })
                ]
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
            indices: 0
        }
    }

    /// Prepare for rendering this frame; create all resources that will be
    /// used during the next render that do not already exist.
    pub fn prepare<V: bytemuck::Pod>(
        &mut self,
        device: &Device,
        queue: &Queue,
        vertices: Vec<[V; 4]>,
    ) {

        let (vertices, indices) = vertices.into_iter().fold(
            (vec![], vec![]), |mut a, vertices| {
                let l = a.0.len() as u16;
                a.0.extend(vertices);
                a.1.extend([l, l+1, l+2, l+1, l+2, l+3]);
                a
            }
        );

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
