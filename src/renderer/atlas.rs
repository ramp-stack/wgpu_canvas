use wgpu::{BindGroup, TextureViewDescriptor, TexelCopyBufferLayout, TextureAspect, Origin3d, TextureUsages, TexelCopyTextureInfo, Extent3d, TextureDimension, TextureDescriptor, TextureFormat, BindGroupLayout, Device, Queue, Sampler};

use crate::{Text, Font, Color, RgbaImage, ShapeType};

use std::collections::HashMap;
use std::sync::Arc;

#[derive(Default, Debug)]
pub struct ImageAtlas(Vec<(Arc<RgbaImage>, Arc<BindGroup>)>);

impl ImageAtlas {
    pub fn trim(&mut self) {
        self.0 = self.0.drain(..).filter(|(i, _)| (Arc::strong_count(i) > 1)).collect();
    }

    pub fn get(
        &mut self,
        queue: &Queue,
        device: &Device,
        layout: &BindGroupLayout,
        sampler: &Sampler,
        image: &Arc<RgbaImage>
    ) -> Arc<BindGroup> {
        match self.0.iter().find_map(|(i, b)| Arc::ptr_eq(i, image).then_some(b)) {
            Some(bind_group) => bind_group.clone(),
            None => {
                let size = Extent3d {
                    width: image.width(),
                    height: image.height(),
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
                    image,
                    TexelCopyBufferLayout{
                        offset: 0,
                        bytes_per_row: Some(4 * image.width()),
                        rows_per_image: Some(image.height()),
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

                self.0.push((image.clone(), bind_group.clone()));
                bind_group
            }
        }
    }
}

type ImageMap = HashMap<char, Option<Arc<RgbaImage>>>;
type Offset = (f32, f32);

#[derive(Default)]
pub struct TextAtlas{
    fonts: Vec<(Arc<Font>, ImageMap)>
}

impl TextAtlas {
    pub fn trim(&mut self) {
        self.fonts = self.fonts.drain(..).filter(|(k, _)| (Arc::strong_count(k) > 1)).collect();
    }

    fn get_font(&mut self, font: &Arc<Font>) -> &mut HashMap<char, Option<Arc<RgbaImage>>> {
        let position = self.fonts.iter().position(|(f, _)| Arc::ptr_eq(f, font)).unwrap_or_else(|| {
            self.fonts.push((font.clone(), HashMap::new()));
            self.fonts.len()-1
        });
        &mut self.fonts[position].1
    }

    fn get_image(&mut self, font: &Arc<Font>, c: char) -> Option<Arc<RgbaImage>> {
        let map = self.get_font(font);
        map.entry(c).or_insert_with(|| {
            let (m, b) = font.rasterize(c, 160.0);//3x bigger than needs to be
            let b: Vec<_> = b.iter().flat_map(|a| [0, 0, 0, *a]).collect();
            let image = b.iter().any(|a| *a != 0).then(|| {
                Arc::new(RgbaImage::from_raw(m.width as u32, m.height as u32, b).unwrap())
            });
            image
        }).clone()
    }

    pub fn get(&mut self, text: Text) -> Vec<(Offset, ShapeType, Arc<RgbaImage>, Color)> {
        text.lines().iter().flat_map(|line| line.2.iter().flat_map(|ch| {
            self.get_image(&ch.2, ch.0).map(|img| {
                let shape = ShapeType::Rectangle(0.0, (ch.1.2, ch.1.3), 0.0);
                let offset = (ch.1.0, ch.1.1);
                (offset, shape, img, ch.3)
            })
        }).collect::<Vec<_>>()).collect::<Vec<_>>()
    } 
}
