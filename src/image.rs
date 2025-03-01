use wgpu::{PipelineCompilationOptions, BindGroupLayoutDescriptor, RenderPipelineDescriptor, PipelineLayoutDescriptor, TextureViewDescriptor, TextureViewDimension, BindGroupLayoutEntry, SamplerBindingType, DepthStencilState, TextureSampleType, TextureDescriptor, TextureDimension, TexelCopyTextureInfo, MultisampleState, BindGroupLayout, TexelCopyBufferLayout, RenderPipeline, PrimitiveState, FragmentState, TextureFormat, TextureUsages, TextureAspect, ShaderStages, BufferUsages, IndexFormat, VertexState, BindingType, RenderPass, BindGroup, Origin3d, Extent3d, Sampler, Device, Queue, ShaderModule};
use wgpu_dyn_buffer::{DynamicBufferDescriptor, DynamicBuffer};

pub use image;

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use std::hash::{DefaultHasher, Hasher, Hash};
use std::sync::Arc;

use crate::shape::{ShapeType, Shape, Vertex, Ellipse, Rectangle, RoundedRectangle};
use super::Area;

pub type ImageKey = u64;

#[derive(Clone, Copy, Debug)]
pub struct ImageShape(pub ImageKey, pub ShapeType);

#[derive(Default, Debug)]
pub struct ImageAtlas {
    uncached: HashMap<ImageKey, image::RgbaImage>,
    cached: HashMap<ImageKey, Arc<BindGroup>>
}

impl ImageAtlas {
    pub fn add(&mut self, image: image::RgbaImage) -> ImageKey {
        let mut hasher = DefaultHasher::new();
        image.hash(&mut hasher);
        let key = hasher.finish();
        self.uncached.insert(key, image);
        key
    }

    pub fn remove(&mut self, key: &ImageKey) {
        self.uncached.remove(key);
        self.cached.remove(key);
    }

    pub fn contains(&self, key: &ImageKey) -> bool {
        self.uncached.contains_key(key) || self.cached.contains_key(key)
    }

    fn prepare(
        &mut self,
        queue: &Queue,
        device: &Device,
        layout: &BindGroupLayout,
        sampler: &Sampler
    ) {
        self.uncached.drain().collect::<Vec<_>>().into_iter().for_each(|(key, image)| {
            if let Entry::Vacant(entry) = self.cached.entry(key) {
                let mut dimensions = image.dimensions();
                dimensions.0 = dimensions.0.min(dimensions.1);
                dimensions.1 = dimensions.0.min(dimensions.1);
                let size = Extent3d {
                    width: dimensions.0,
                    height: dimensions.1,
                    depth_or_array_layers: 1,
                };

                let texture = device.create_texture(
                    &TextureDescriptor {
                        size,
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: TextureDimension::D2,
                        format: TextureFormat::Rgba8UnormSrgb,
                        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                        label: None,
                        view_formats: &[],
                    }
                );

                queue.write_texture(
                    TexelCopyTextureInfo {
                        texture: &texture,
                        mip_level: 0,
                        origin: Origin3d::ZERO,
                        aspect: TextureAspect::All,
                    },
                    &image,
                    TexelCopyBufferLayout{
                        offset: 0,
                        bytes_per_row: Some(4 * dimensions.0),
                        rows_per_image: Some(dimensions.1),
                    },
                    size
                );

                let texture_view = texture.create_view(&TextureViewDescriptor::default());

                let bind_group = Arc::new(device.create_bind_group(
                    &wgpu::BindGroupDescriptor {
                        layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&texture_view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(sampler),
                            }
                        ],
                        label: None,
                    }
                ));
                entry.insert(bind_group);
            }
        })
    }

    fn get(&self, key: &ImageKey) -> Arc<BindGroup> {
        self.cached.get(key).expect("Image not found for ImageKey").clone()
    }
}

pub(crate) struct ImageShapeRenderer {
    ellipse_renderer: GenericImageRenderer<Ellipse>,
    rectangle_renderer: GenericImageRenderer<Rectangle>,
    rounded_rectangle_renderer: GenericImageRenderer<RoundedRectangle>,
}

impl ImageShapeRenderer {
    /// Create all unchanging resources here.
    pub fn new(
        device: &Device,
        texture_format: &TextureFormat,
        multisample: MultisampleState,
        depth_stencil: Option<DepthStencilState>,
    ) -> Self {
        let ellipse_shader = device.create_shader_module(wgpu::include_wgsl!("image/ellipse.wgsl"));
        let rectangle_shader = device.create_shader_module(wgpu::include_wgsl!("image/rectangle.wgsl"));
        let rounded_rectangle_shader = device.create_shader_module(wgpu::include_wgsl!("image/rounded_rectangle.wgsl"));
        ImageShapeRenderer{
            ellipse_renderer: GenericImageRenderer::new(device, texture_format, multisample, depth_stencil.clone(), ellipse_shader),
            rectangle_renderer: GenericImageRenderer::new(device, texture_format, multisample, depth_stencil.clone(), rectangle_shader),
            rounded_rectangle_renderer: GenericImageRenderer::new(device, texture_format, multisample, depth_stencil.clone(), rounded_rectangle_shader),
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
        is_area: Vec<(ImageShape, Area)>,
    ) {
        let (el_areas, rect_areas, rrect_areas) = is_area.into_iter().fold(
            (vec![], vec![], vec![]), |mut a, (is, area)| {
                match is.1 {
                    ShapeType::Ellipse(ellipse) =>
                        a.0.push((is.0, ellipse, area)),
                    ShapeType::Rectangle(rectangle) =>
                        a.1.push((is.0, rectangle, area)),
                    ShapeType::RoundedRectangle(rounded_rectangle) =>
                        a.2.push((is.0, rounded_rectangle, area))
                }
                a
            }
        );

        self.ellipse_renderer.prepare(device, queue, width, height, image_atlas, el_areas);
        self.rectangle_renderer.prepare(device, queue, width, height, image_atlas, rect_areas);
        self.rounded_rectangle_renderer.prepare(device, queue, width, height, image_atlas, rrect_areas);
    }

    /// Render using caller provided render pass.
    pub fn render(&self, render_pass: &mut RenderPass<'_>) {
        self.ellipse_renderer.render(render_pass);
        self.rectangle_renderer.render(render_pass);
        self.rounded_rectangle_renderer.render(render_pass);
    }
}

struct GenericImageRenderer<S: Shape> {
    render_pipeline: RenderPipeline,
    vertex_buffer: DynamicBuffer,
    index_buffer: DynamicBuffer,
    bind_group_layout: BindGroupLayout,
    sampler: Sampler,
    indices: HashMap<Arc<BindGroup>, Vec<(u32, u32)>>,
    _marker: std::marker::PhantomData<S>
}

impl<S: Shape> GenericImageRenderer<S> {
    /// Create all unchanging resources here.
    pub fn new(
        device: &Device,
        texture_format: &TextureFormat,
        multisample: MultisampleState,
        depth_stencil: Option<DepthStencilState>,
        shader: ShaderModule
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
                        sample_type: TextureSampleType::Float{filterable: true},
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ]
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor{
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

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

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        GenericImageRenderer{
            render_pipeline,
            vertex_buffer,
            index_buffer,
            bind_group_layout,
            sampler,
            indices: HashMap::new(),
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
        image_atlas: &mut ImageAtlas,
        img_sh_area: Vec<(ImageKey, S, Area)>
    ) {
        self.indices.clear();

        image_atlas.prepare(queue, device, &self.bind_group_layout, &self.sampler);

        let (vertices, indices, indices_buffer) = img_sh_area.into_iter().fold(
            (vec![], vec![], HashMap::<Arc<BindGroup>, Vec<(u32, u32)>>::new()),
            |mut a, (key, shape, area)| {
                let image = image_atlas.get(&key);
                let start = a.1.len();
                let (v, i) = shape.build(width, height, area, a.0.len() as u16);
                a.0.extend(v);
                a.1.extend(i);
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
