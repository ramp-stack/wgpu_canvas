use wgpu::{PipelineCompilationOptions, BindGroupLayoutDescriptor, RenderPipelineDescriptor, PipelineLayoutDescriptor, TextureViewDimension, BindGroupLayoutEntry, DepthStencilState, TextureSampleType, MultisampleState, BindGroupLayout, RenderPipeline, PrimitiveState, FragmentState, TextureFormat, ShaderStages, BufferUsages, IndexFormat, VertexState, BindingType, RenderPass, BindGroup, Device, Queue, VertexBufferLayout, VertexStepMode, BufferAddress};
use wgpu_dyn_buffer::{DynamicBufferDescriptor, DynamicBuffer};

use std::sync::Arc;
use std::collections::HashMap;
use crate::Area;
use super::{ImageAtlas, ImageKey};


#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct ShapeVertex {
    pub uv: [f32; 2],
    pub position: [f32; 2],
    pub bounds: [f32; 4],
    pub z_index: f32
}

impl ShapeVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 4] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4, 3 => Float32];

    fn layout() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

pub struct ImageRenderer {
    render_pipeline: RenderPipeline,
    vertex_buffer: DynamicBuffer,
    index_buffer: DynamicBuffer,
    bind_group_layout: BindGroupLayout,
    indices: HashMap<Arc<BindGroup>, Vec<(u32, u32)>>,
}

impl ImageRenderer {
    /// Create all unchanging resources here.
    pub fn new(
        device: &Device,
        texture_format: &TextureFormat,
        multisample: MultisampleState,
        depth_stencil: Option<DepthStencilState>,
    ) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor{
            label: None,
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float{filterable: false},
                    },
                    count: None,
                }
            ]
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor{
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[ShapeVertex::layout()]
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some((*texture_format).into())],
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

        ImageRenderer{
            render_pipeline,
            vertex_buffer,
            index_buffer,
            bind_group_layout,
            indices: HashMap::new(),
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
        image_atlas: &mut ImageAtlas,
        img_area: Vec<(ImageKey, Area)>,
    ) {
        self.indices.clear();

        image_atlas.prepare(queue, device, &self.bind_group_layout);

        let w = |x: f32| ((x / width as f32) * 2.0) - 1.0;
        let h = |y: f32| 1.0 - ((y / height as f32) * 2.0);

        let (vertices, indices, indices_buffer) = img_area.into_iter().fold(
            (vec![], vec![], HashMap::<Arc<BindGroup>, Vec<(u32, u32)>>::new()),
            |mut a, (key, area)| {
                let (image, size) = image_atlas.get(&key);
                let start = a.1.len();

                let x = w(area.offset.0 as f32);
                let y = h(area.offset.1 as f32);
                let x2 = w((area.offset.0+size.0) as f32);
                let y2 = h((area.offset.1+size.1) as f32);

                let bx = (area.bounds.0 as f32 - area.offset.0 as f32) - size.0 as f32;
                let by = (area.bounds.1 as f32 - area.offset.1 as f32) - size.1 as f32;
                let bx2 = ((area.bounds.0+area.bounds.2) as f32 - area.offset.0 as f32) - size.0 as f32;
                let by2 = ((area.bounds.1+area.bounds.3) as f32 - area.offset.1 as f32) - size.1 as f32;
                let bounds = [bx, by, bx2, by2];

                let z_index = area.z_index as f32 / u16::MAX as f32;

                let l = a.0.len() as u16;
                a.0.extend([
                        ShapeVertex{uv: [0.0, 0.0], position: [x, y], bounds, z_index},
                        ShapeVertex{uv: [size.0 as f32, 0.0], position: [x2, y], bounds, z_index},
                        ShapeVertex{uv: [0.0, size.1 as f32], position: [x, y2], bounds, z_index},
                        ShapeVertex{uv: [size.0 as f32, size.1 as f32], position: [x2, y2], bounds, z_index}
                ]);

                a.1.extend([l, l+1, l+2, l+1, l+2, l+3]);

                let index = (start as u32, a.1.len() as u32);
                match a.2.get_mut(&image) {
                    Some(indices) => indices.push(index),
                    None => {a.2.insert(image, vec![index]);}
                }
                a
            }
        );

        self.indices = indices_buffer;
        self.vertex_buffer.write_buffer(device, queue, bytemuck::cast_slice(&vertices));
        self.index_buffer.write_buffer(device, queue, bytemuck::cast_slice(&indices));
    }

    /// Render using caller provided render pass.
    pub fn render(&self, render_pass: &mut RenderPass<'_>) {
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.as_ref().slice(..));
        render_pass.set_index_buffer(self.index_buffer.as_ref().slice(..), IndexFormat::Uint16);
        for (bind_group, indices) in &self.indices {
            render_pass.set_bind_group(0, Some(&**bind_group), &[]);
            for (start, end) in indices {
                render_pass.draw_indexed(*start..*end, 0, 0..1);
            }
        }
    }
}
