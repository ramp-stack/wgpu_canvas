use wgpu::{BindGroup, TextureViewDescriptor, TexelCopyBufferLayout, TextureAspect, Origin3d, TextureUsages, TexelCopyTextureInfo, Extent3d, TextureDimension, TextureDescriptor, TextureFormat, BindGroupLayout, Device, Queue, Sampler};

pub use image::RgbaImage;

use std::hash::{DefaultHasher, Hasher, Hash};
use std::collections::BTreeMap;
use std::sync::Arc;

mod renderer;
pub(crate) use renderer::ImageRenderer;

#[derive(Clone, Debug, Ord, PartialOrd, PartialEq, Eq)]
pub struct Image(Arc<u64>, u32, u32);

impl Image {
    pub fn size(&self) -> (u32, u32) {(self.1, self.2)}
}

pub type InnerImage = Arc<BindGroup>;

#[derive(Debug)]
pub struct ImageAtlas(Option<BTreeMap<Image, (RgbaImage, Option<InnerImage>)>>);

impl ImageAtlas {
    pub fn add(&mut self, raw: RgbaImage) -> Image {
        let size = raw.dimensions();
        let mut hasher = DefaultHasher::new();
        raw.hash(&mut hasher);
        let key = hasher.finish();

        let image = Image(Arc::new(key), size.0, size.1);
        match self.0.as_mut().unwrap().get_key_value(&image) {
            Some((image, _)) => image.clone(),
            None => {
                self.0.as_mut().unwrap().insert(image.clone(), (raw, None));
                image
            }
        }
    }

    pub(crate) fn trim_and_bind(
        &mut self,
        queue: &Queue,
        device: &Device,
        layout: &BindGroupLayout,
        sampler: &Sampler
    ) {
        self.0 = Some(self.0.take().unwrap().into_iter().filter_map(|(image, v)|
            //TODO: use match Arc::strong_count instead of try_unwrap, And .is_some instead of unwrap_or_else
            Arc::try_unwrap(image.0).err().map(|k| {
                let inner_image = v.1.unwrap_or_else(|| {
                    let size = Extent3d {
                        width: image.1,
                        height: image.2,
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
                        &v.0,
                        TexelCopyBufferLayout{
                            offset: 0,
                            bytes_per_row: Some(4 * image.1),
                            rows_per_image: Some(image.2),
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
                                },
                                wgpu::BindGroupEntry {
                                    binding: 1,
                                    resource: wgpu::BindingResource::Sampler(sampler),
                                }
                            ],
                            label: None,
                        }
                    ))
                });
                (Image(k, image.1, image.2), (v.0, Some(inner_image)))
            })
        ).collect());
    }

    pub(crate) fn get(&self, key: &Image) -> InnerImage {
        self.0.as_ref().unwrap().get(key).as_ref().unwrap().1.clone().unwrap()
    }
}
impl Default for ImageAtlas {fn default() -> Self {ImageAtlas(Some(BTreeMap::new()))}}
