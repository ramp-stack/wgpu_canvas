use glyphon::{Resolution, SwashCache, FontSystem, TextBounds, TextAtlas, Viewport, Metrics, Shaping, Buffer, Family, Cache, Attrs, Wrap};
use wgpu::{DepthStencilState, MultisampleState, TextureFormat, RenderPass, Device, Queue};
use glyphon::fontdb::{Database, Source, ID};

use std::sync::Arc;
use std::collections::HashMap;

use super::{Area, Color};

#[derive(Clone, Debug)]
pub struct Text {
    pub text: String,
    pub color: Color,
    pub width: Option<f32>,
    pub size: f32,
    pub line_height: f32,
    pub font: Font,
}

impl Text {
    fn to_buffer(&self, font_system: &mut FontSystem, metadata: usize) -> Buffer {
        let font_attrs = self.font.1.metadata(metadata);
        let metrics = Metrics::new(self.size as f32, self.line_height as f32);
        let mut buffer = Buffer::new(font_system, metrics);
        buffer.set_wrap(font_system, Wrap::WordOrGlyph);
        buffer.set_size(font_system, self.width.map(|w| 1.0+w as f32), None);
        buffer.set_text(font_system, &self.text, font_attrs, Shaping::Basic);
        buffer
    }
}

pub type Font = Arc<(ID, Attrs<'static>)>;

pub struct FontAtlas{
    fonts: Option<HashMap<Arc<Vec<u8>>, Font>>,
    font_system: FontSystem
}

impl FontAtlas {
    pub fn measure_text(&mut self, text: &Text) -> (f32, f32) {
        text.to_buffer(&mut self.font_system, 0).lines.first().map(|line|
            line.layout_opt().as_ref().unwrap().iter().fold((0.0f32, 0.0f32), |(w, h), span| {
                (w.max(span.w) as f32, h+span.line_height_opt.unwrap_or(span.max_ascent+span.max_descent) as f32)
            })
        ).unwrap_or((0.0, 0.0))
    }

    pub fn add(&mut self, raw_font: Vec<u8>) -> Font {
        let raw_font = Arc::new(raw_font);
        match self.fonts.as_mut().unwrap().get(&raw_font) {
            Some(font) => font.clone(),
            None => {
                let database = self.font_system.db_mut();
                let id = database.load_font_source(Source::Binary(raw_font.clone()))[0];
                let face = database.face(id).unwrap();
                let attrs = Attrs::new()
                    .family(Family::<'static>::Name(face.families[0].0.clone().leak()))
                    .stretch(face.stretch)
                    .style(face.style)
                    .weight(face.weight);
                let font = Arc::new((id, attrs));
                self.fonts.as_mut().unwrap().insert(raw_font, font.clone());
                font
            }
        }
    }

    fn trim(&mut self) {
        let to_remove = self.fonts.as_ref().unwrap().iter().filter_map(|(k, v)| (Arc::strong_count(&v) > 1).then(|| k.clone())).collect::<Vec<_>>();
        to_remove.into_iter().for_each(|k| {self.fonts.as_mut().unwrap().remove(&k);});
    }
}

impl Default for FontAtlas {fn default() -> Self {
    FontAtlas{
        fonts: Some(HashMap::new()),
        font_system: FontSystem::new_with_locale_and_db("".to_string(), Database::new())
    }
}}

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
        width: f32,
        height: f32,
        font_atlas: &mut FontAtlas,
        text_areas: Vec<(Area, Text)>
    ) {
        font_atlas.trim();
        self.text_atlas.trim();
        self.viewport.update(queue, Resolution{width: width.floor() as u32, height: height.floor() as u32});

        let buffers = text_areas.iter().map(|(a, t)|
            t.to_buffer(&mut font_atlas.font_system, a.z_index as usize)
        ).collect::<Vec<_>>();

        let text_areas = text_areas.into_iter().zip(buffers.iter()).map(|((a, t), b)| {
            let left = a.bounds.0;
            let top = a.bounds.1;
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
                default_color: glyphon::Color::rgba(t.color.0, t.color.1, t.color.2, t.color.3),
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
