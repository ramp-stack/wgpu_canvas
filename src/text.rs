use glyphon::{Resolution, SwashCache, FontSystem, TextBounds, TextAtlas, Viewport, Metrics, Shaping, Buffer, Family, Color, Cache, Attrs, Wrap};
use wgpu::{DepthStencilState, MultisampleState, TextureFormat, RenderPass, Device, Queue};
use glyphon::fontdb::{Database, Source};

use super::Area;

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::hash::{DefaultHasher, Hasher, Hash};
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub struct Text {
    pub text: &'static str,
    pub color: (u8, u8, u8, u8),
    pub width: Option<u32>,
    pub size: u32,
    pub line_height: u32,
    pub font: FontKey,
}

impl Text {
    pub fn new(
        text: &'static str,
        color: (u8, u8, u8, u8),
        width: Option<u32>,
        size: u32,
        line_height: u32,
        font: FontKey,
    ) -> Self {
        Text{text, color, width, size, line_height, font}
    }

    fn into_buffer(self, font_atlas: &mut FontAtlas, metadata: usize) -> Buffer {
        let font = font_atlas.get(&self.font).metadata(metadata);
        let metrics = Metrics::new(self.size as f32, self.line_height as f32);
        let mut buffer = Buffer::new(&mut font_atlas.font_system, metrics);
        buffer.set_wrap(&mut font_atlas.font_system, Wrap::WordOrGlyph);
        buffer.set_size(&mut font_atlas.font_system, self.width.map(|w| w as f32), None);
        buffer.set_text(&mut font_atlas.font_system, self.text, font, Shaping::Advanced);
        buffer
    }
}

pub type FontKey = u64;

pub struct FontAtlas {
    fonts: HashMap<FontKey, Arc<Attrs<'static>>>,
    font_system: FontSystem,
}

impl FontAtlas {
    pub fn new() -> Self {
        FontAtlas{
            fonts: HashMap::new(),
            font_system: FontSystem::new_with_locale_and_db("".to_string(), Database::new())
        }
    }

    pub fn add(&mut self, font: Vec<u8>) -> FontKey {
        let mut hasher = DefaultHasher::new();
        font.hash(&mut hasher);
        let key = hasher.finish();

        if let Entry::Vacant(entry) = self.fonts.entry(key) {
            let database = self.font_system.db_mut();
            let id = database.load_font_source(Source::Binary(Arc::new(font)))[0];
            let face = database.face(id).unwrap();
            let attrs = Attrs::new()
                .family(Family::<'static>::Name(face.families[0].0.clone().leak()))
                .stretch(face.stretch)
                .style(face.style)
                .weight(face.weight);
            entry.insert(Arc::new(attrs));
        }
        key
    }

    pub fn remove(&mut self, key: &FontKey) {self.fonts.remove(key);}
    pub fn contains(&self, key: &FontKey) -> bool {self.fonts.contains_key(key)}

    pub fn messure_text(&mut self, t: &Text) -> (u32, u32) {
        t.into_buffer(self, 0).lines.first().map(|line|
            line.layout_opt().as_ref().unwrap().iter().fold((0, 0), |(w, h), span| {
                (w.max(span.w as u32), h+span.line_height_opt.unwrap_or(span.max_ascent+span.max_descent) as u32)
            })
        ).unwrap_or((0, 0))
    }

    fn get(&self, key: &FontKey) -> Arc<Attrs<'static>> {
        self.fonts.get(key).expect("Font could not be found for Key").clone()
    }
}
impl Default for FontAtlas {fn default() -> Self {Self::new()}}

pub struct TextRenderer {
    text_renderer: glyphon::TextRenderer,
    swash_cache: SwashCache,
    text_atlas: TextAtlas,
    viewport: Viewport,
}

impl TextRenderer {
    pub fn new(
        device: &Device,
        queue: &Queue,
        texture_format: &TextureFormat,
        multisample: MultisampleState,
        depth_stencil: Option<DepthStencilState>,
    ) -> Self {
        let cache = Cache::new(device);
        let mut text_atlas = TextAtlas::new(device, queue, &cache, *texture_format);
        let text_renderer = glyphon::TextRenderer::new(&mut text_atlas, device, multisample, depth_stencil);

        TextRenderer{
            text_renderer,
            text_atlas,
            viewport: Viewport::new(device, &cache),
            swash_cache: SwashCache::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn prepare(
        &mut self,
        device: &Device,
        queue: &Queue,
        width: u32,
        height: u32,
        font_atlas: &mut FontAtlas,
        text_areas: Vec<(Text, Area)>
    ) {
        self.text_atlas.trim();
        self.viewport.update(queue, Resolution{width, height});

        let buffers = text_areas.iter().map(|(t, a)|
            t.into_buffer(font_atlas, a.z_index as usize)
        ).collect::<Vec<_>>();

        let text_areas = text_areas.into_iter().zip(buffers.iter()).map(|((t, a), b)| {
            let left = a.bounds.0 as i32;
            let top = a.bounds.1 as i32;
            glyphon::TextArea{
                buffer: b,
                left: a.offset.0 as f32,
                top: a.offset.1 as f32,
                scale: 1.0,
                bounds: TextBounds {//Sisscor Rect
                    left,
                    top,
                    right: left + a.bounds.2 as i32,
                    bottom: top + a.bounds.3 as i32,
                },
                default_color: Color::rgba(t.color.0, t.color.1, t.color.2, t.color.3),
                custom_glyphs: &[]
            }
        });

        self.text_renderer.prepare_with_depth(
            device,
            queue,
            &mut font_atlas.font_system,
            &mut self.text_atlas,
            &self.viewport,
            text_areas,
            &mut self.swash_cache,
            |z: usize| ((z as u16) as f32) / u16::MAX as f32
        ).unwrap();
    }

    pub fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        self.text_renderer.render(&self.text_atlas, &self.viewport, render_pass).unwrap();
    }
}
