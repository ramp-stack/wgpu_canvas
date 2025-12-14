use wgpu::{RenderPassDepthStencilAttachment, RenderPassColorAttachment, CommandEncoderDescriptor, TextureViewDescriptor, RequestAdapterOptions, SurfaceConfiguration, RenderPassDescriptor, InstanceDescriptor, DepthStencilState, TextureDescriptor, TextureDimension, MultisampleState, DeviceDescriptor, PowerPreference, CompareFunction, WindowHandle, DepthBiasState, TextureUsages, TextureFormat, StencilState, TextureView, Operations, Instance, Features, Extent3d, Surface, StoreOp, LoadOp, Limits, Device, Queue, Trace};

use std::sync::Arc;

use crate::{Renderer, Atlas, Area, Item};

const SAMPLE_COUNT: u32 = 4;

pub struct Canvas {
    _instance: Instance,
    surface: Surface<'static>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    msaa_view: Option<TextureView>,
    depth_view: TextureView,
    renderer: Renderer,
}

impl Canvas {
    /// Creates a new `Canvas` for the given window and size.
    ///
    /// Returns the `Canvas` and its initial `(width, height)`
    pub async fn new<W: WindowHandle + 'static>(window: W, width: u32, height: u32) -> (Self, (u32, u32)) {
        let instance = Instance::new(&InstanceDescriptor::default());

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance.request_adapter(
            &RequestAdapterOptions {
                power_preference: PowerPreference::None,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ).await.unwrap();

        let mut limits = Limits::downlevel_webgl2_defaults();
        limits.max_texture_dimension_2d = if cfg!(target_os = "android") {4096} else {8192};

        let width = width.min(limits.max_texture_dimension_2d);
        let height = height.min(limits.max_texture_dimension_2d);

        let (device, queue) = adapter.request_device(
            &DeviceDescriptor {
                required_features: Features::empty(),
                required_limits: limits,
                label: None,
                memory_hints: Default::default(),
                trace: Trace::Off
            }
        ).await.unwrap();

        let surface_caps = surface.get_capabilities(&adapter);

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            width,
            height,
            format: surface_caps.formats[0],
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![surface_caps.formats[0]],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        let multisample = MultisampleState {
            count: SAMPLE_COUNT,
            mask: !0,
            alpha_to_coverage_enabled: true,
        };

        let depth_stencil = DepthStencilState {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: CompareFunction::GreaterEqual,
            stencil: StencilState::default(),
            bias: DepthBiasState::default(),
        };

        let msaa_view = (SAMPLE_COUNT > 1).then(|| Self::create_msaa_view(&device, &config));

        let depth_view = Self::create_depth_view(&device, &config);

        let renderer = Renderer::new(&device, &surface_caps.formats[0], multisample, Some(depth_stencil));

        let size = (config.width, config.height);

        (Canvas{
            _instance: instance,
            surface,
            device,
            queue,
            config,
            msaa_view,
            depth_view,
            renderer,
        }, size)
    }

    /// Resizes the canvas to the given dimensions.
    ///
    /// Returns the updated `(width, height)`.
    pub fn resize<W: WindowHandle + 'static>(
        &mut self, _new_window: Option<Arc<W>>, width: u32, height: u32
    ) -> (u32, u32) {
        // if let Some(new_window) = new_window {
        //     self.surface = self.instance.create_surface(new_window).unwrap();
        // }

        if width > 0 && height > 0 {
            let limits = self.device.limits();
            self.config.width = width.min(limits.max_texture_dimension_2d);
            self.config.height = height.min(limits.max_texture_dimension_2d);
            self.surface.configure(&self.device, &self.config);
            if SAMPLE_COUNT > 1 {
                self.msaa_view = Some(Self::create_msaa_view(&self.device, &self.config));
            }
            self.depth_view = Self::create_depth_view(&self.device, &self.config);
        }

        (self.config.width, self.config.height)
    }

    /// Draws the given `items` using the provided `atlas`.
    ///
    /// Handles render pass setup, MSAA, and depth buffer automatically.
    pub fn draw(&mut self, atlas: &mut Atlas, items: Vec<(Area, Item)>) {
        self.renderer.prepare(
            &self.device,
            &self.queue,
            self.config.width as f32,
            self.config.height as f32,
            atlas, items
        );

        let output = self.surface.get_current_texture().unwrap();
        let frame_view = output.texture.create_view(&TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor::default());
        let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: if SAMPLE_COUNT > 1 {self.msaa_view.as_ref().unwrap()} else {&frame_view},
                resolve_target: if SAMPLE_COUNT > 1 {Some(&frame_view)} else {None},
                ops: Operations {
                    load: LoadOp::Clear(wgpu::Color::BLACK),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &self.depth_view,
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(0.0),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        self.renderer.render(&mut rpass);

        drop(rpass);

        self.queue.submit(Some(encoder.finish()));
        output.present();
    }

    fn create_msaa_view(device: &Device, config: &SurfaceConfiguration) -> TextureView {
        device.create_texture(&TextureDescriptor{
            label: Some("Multisampled frame descriptor"),
            size: Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: SAMPLE_COUNT,
            dimension: TextureDimension::D2,
            format: config.format,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&TextureViewDescriptor::default())
    }

    fn create_depth_view(device: &Device, config: &SurfaceConfiguration) -> TextureView {
        device.create_texture(&TextureDescriptor {
            label: Some("Depth Stencil Texture"),
            size: Extent3d { // 2.
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: SAMPLE_COUNT,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&TextureViewDescriptor::default())
    }
}
