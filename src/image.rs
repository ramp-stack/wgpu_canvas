use wgpu::{BindGroup, TextureViewDescriptor, TexelCopyBufferLayout, TextureAspect, Origin3d, TextureUsages, TexelCopyTextureInfo, Extent3d, TextureDimension, TextureDescriptor, TextureFormat, BindGroupLayout, Device, Queue};

pub use image::RgbaImage;

use std::collections::HashMap;
use std::sync::Arc;

mod renderer;
pub(crate) use renderer::ImageRenderer;

pub type Image = Arc<RgbaImage>;

pub type InnerImage = Arc<BindGroup>;//, (u32, u32));

#[derive(Debug)]
pub struct ImageAtlas(Option<HashMap<Image, Option<InnerImage>>>);

impl ImageAtlas {
    pub fn add(&mut self, image: RgbaImage) -> Image {
        let image = Arc::new(image);
        match self.0.as_mut().unwrap().get(&image) {
            Some(_) => image.clone(),
            None => {
                self.0.as_mut().unwrap().insert(image.clone(), None);
                image
            }
        }
    }

    pub fn trim_and_bind(
        &mut self,
        queue: &Queue,
        device: &Device,
        layout: &BindGroupLayout,
    ) {
        self.0 = Some(self.0.take().unwrap().into_iter().filter_map(|(k, v)| Arc::try_unwrap(k).err().map(|image| {
            let inner_image = v.unwrap_or_else(|| {
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

                Arc::new(device.create_bind_group(
                    &wgpu::BindGroupDescriptor {
                        layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&texture_view),
                            }
                        ],
                        label: None,
                    }
                ))
            });
            (image, Some(inner_image))
        })
        ).collect());
    }

    pub fn get(&self, key: &Image) -> InnerImage {
        self.0.as_ref().unwrap().get(key).expect("Image not found in Atlas").clone().unwrap()
    }
}
impl Default for ImageAtlas {fn default() -> Self {ImageAtlas(Some(HashMap::new()))}}
