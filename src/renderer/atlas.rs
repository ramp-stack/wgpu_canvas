use wgpu::{BindGroup, TextureViewDescriptor, TexelCopyBufferLayout, TextureAspect, Origin3d, TextureUsages, TexelCopyTextureInfo, Extent3d, TextureDimension, TextureDescriptor, TextureFormat, BindGroupLayout, Device, Queue, Sampler};

use crate::shape::Shape;
use crate::text::EMOJI_FONT;
use crate::{Text, Font, Color, RgbaImage};

use std::collections::HashMap;
use std::sync::Arc;

use swash::shape::ShapeContext;
use swash::scale::image::Content;
use swash::scale::{Render, ScaleContext, Source, StrikeWith};

#[derive(Default, Debug)]
pub struct Atlas {
    pub(crate) image: ImageAtlas,
    pub(crate) text: TextAtlas,
}

impl Atlas {
    pub(crate) fn trim(&mut self) {
        self.image.trim();
        self.text.trim();
    }
}

#[derive(Default, Debug)]
pub struct ImageAtlas(Vec<(Arc<RgbaImage>, Arc<BindGroup>)>);

impl ImageAtlas {
    pub fn trim(&mut self) {
        self.0 = self.0.drain(..).filter(|(i, _)| Arc::strong_count(i) > 1).collect();
    }

    pub fn get(
        &mut self,
        queue: &Queue,
        device: &Device,
        layout: &BindGroupLayout,
        sampler: &Sampler,
        image: &Arc<RgbaImage>
    ) -> Arc<BindGroup> {
        self.0.iter().find_map(|(i, b)| Arc::ptr_eq(i, image).then_some(b)).cloned().unwrap_or_else(|| {
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
        }).clone()
    }
}

type ImageMap = HashMap<String, Option<Arc<RgbaImage>>>;
type Offset = (f32, f32);

#[derive(Default, Debug)]
pub struct TextAtlas {
    fonts: Vec<(Font, ImageMap)>
}

impl TextAtlas {
    pub fn trim(&mut self) {
        self.fonts = self.fonts.drain(..).filter(|(k, _)| Arc::strong_count(&k.0) > 1).collect();
    }

    fn get_font(&mut self, font: &Font) -> &mut HashMap<String, Option<Arc<RgbaImage>>> {
        let position = self.fonts.iter().position(|(f, _)| Arc::ptr_eq(&f.0, &font.0)).unwrap_or_else(|| {
            self.fonts.push((font.clone(), HashMap::new()));
            self.fonts.len() - 1
        });

        &mut self.fonts[position].1
    }

    fn get_swash_image(&mut self, ch: String) -> Option<Arc<RgbaImage>> {
        self.get_font(&EMOJI_FONT).entry(ch.clone()).or_insert_with(|| {
            let render = Render::new(&[
                Source::ColorBitmap(StrikeWith::BestFit),
                Source::ColorOutline(0),
                Source::Outline,
            ]);

            let mut glyph_id = None;
            let mut context = ShapeContext::new();
            let mut shaper = context
                .builder(EMOJI_FONT.swash_font())
                .size(160.0).build();
            
            shaper.add_str(&ch);
            shaper.shape_with(|cluster| 
                if glyph_id.is_none() && let Some(glyph) = cluster.glyphs.first() {
                    glyph_id = Some(glyph.id);
                }
            );

            let mut ctx = ScaleContext::new();
            let image = render.render(
                &mut ctx.builder(EMOJI_FONT.swash_font()).size(160.0).hint(false).build(), 
                glyph_id.unwrap()
            )?;
            
            if image.placement.width == 0 || image.placement.height == 0 { return None; }

            Some(Arc::new(RgbaImage::from_raw(
                image.placement.width, 
                image.placement.height, 
                match image.content {
                    Content::Mask => image.data.iter().flat_map(|a| [0, 0, 0, *a]).collect(),
                    Content::Color | Content::SubpixelMask => image.data,
                }
            )?))
        }).clone()
    }

    fn get_image(&mut self, font: &Font, ch: String) -> Option<Arc<RgbaImage>> {
        if crate::text::is_emoji_grapheme(&ch) { return self.get_swash_image(ch); }
        self.get_font(font).entry(ch.clone()).or_insert_with(|| {
            let ch_char = ch.chars().next()?;
            let (m, b) = font.rasterize(ch_char, 160.0);
            let b: Vec<_> = b.iter().flat_map(|a| [0, 0, 0, *a]).collect();

            b.iter().any(|a| *a != 0).then(|| Arc::new(
                RgbaImage::from_raw(m.width as u32, m.height as u32,b).unwrap()
            ))
        }).clone()
    }

    pub fn get(&mut self, text: Text) -> Vec<(Offset, Shape, Arc<RgbaImage>, Option<Color>)> {
        let mut result = Vec::new();

        text.lines().iter().for_each(|line| line.2.iter().for_each(|ch| {
            if let Some(img) = self.get_image(&ch.2, ch.0.clone()) {
                let shape = Shape::Rectangle(0.0, (ch.1.2, ch.1.3), 0.0);
                result.push(((ch.1.0, ch.1.1), shape, img, ch.3));
            }
        }));

        result
    }
}
