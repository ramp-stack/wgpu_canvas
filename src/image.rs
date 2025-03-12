use wgpu::{BindGroup, TextureViewDescriptor, TexelCopyBufferLayout, TextureAspect, Origin3d, TextureUsages, TexelCopyTextureInfo, Extent3d, TextureDimension, TextureDescriptor, TextureFormat, BindGroupLayout, Device, Queue};

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use std::hash::{DefaultHasher, Hasher, Hash};
use std::sync::Arc;

mod renderer;
pub(crate) use renderer::ImageRenderer;

pub use image_crate::RgbaImage as Image;
pub type ImageKey = u64;

#[derive(Default, Debug)]
pub(crate) struct ImageAtlas {
    uncached: HashMap<ImageKey, Image>,
    cached: HashMap<ImageKey, (Arc<BindGroup>, (u32, u32))>
}

impl ImageAtlas {
    pub fn add(&mut self, image: Image) -> ImageKey {
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

    pub fn prepare(
        &mut self,
        queue: &Queue,
        device: &Device,
        layout: &BindGroupLayout,
    ) {
        self.uncached.drain().collect::<Vec<_>>().into_iter().for_each(|(key, image)| {
            if let Entry::Vacant(entry) = self.cached.entry(key) {

                let dimensions = image.dimensions();
                let size = Extent3d {
                    width: dimensions.0,
                    height: dimensions.1,
                    depth_or_array_layers: 1,
                };

                let texture = device.create_texture(
                    &TextureDescriptor {
                        size,
                        mip_level_count: 1,
                        sample_count: 4,
                        dimension: TextureDimension::D2,
                        format: TextureFormat::Rgba8UnormSrgb,
                        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT,
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
                        entries: &[wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&texture_view),
                        }],
                        label: None,
                    }
                ));
                entry.insert((bind_group, dimensions));
            }
        })
    }

    pub fn get(&self, key: &ImageKey) -> (Arc<BindGroup>, (u32, u32)) {
        self.cached.get(key).expect("Image not found for ImageKey").clone()
    }
}
