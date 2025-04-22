use glyphon::{Resolution, SwashCache, FontSystem, TextBounds, TextAtlas, Viewport, Metrics, Shaping, Buffer, Family, Cache, Attrs, Wrap};
use wgpu::{DepthStencilState, MultisampleState, TextureFormat, RenderPass, Device, Queue};
use glyphon::fontdb::{Database, Source, ID};
use glyphon::cosmic_text::LineEnding;

use std::sync::Arc;
use std::collections::HashMap;

use super::{Area, Color};

#[derive(Clone, Debug)]
pub struct Text{
    buffer: Buffer,
    pub text: String,
    pub color: Color,
    pub font: Font,
    pub size: f32,
    pub line_height: f32,
    pub width: Option<f32>
}

impl Text {
    pub fn new(
        font_system: &mut impl AsMut<FontSystem>,
        text: &str, color: Color, font: Font,
        size: f32, line_height: f32, width: Option<f32>
    ) -> Self {
        let metrics = Metrics::new(size, line_height);
        let mut buffer = Buffer::new(font_system.as_mut(), metrics);
        buffer.set_wrap(font_system.as_mut(), Wrap::WordOrGlyph);
        buffer.set_size(font_system.as_mut(), width.map(|w| 1.0+w), None);
        buffer.set_text(font_system.as_mut(), text, font.1, Shaping::Basic);
        Text{buffer, text: text.to_string(), color, font, size, line_height, width}
    }

  //pub fn set_text(&mut self, font_system: &mut impl AsMut<FontSystem>, text: &str) {
  //    self.0.set_text(font_system.as_mut(), text, self.2.1, Shaping::Basic);
  //}

    pub fn size(&self) -> (f32, f32) {
        let newline = self.buffer.lines.last().and_then(|line| (!self.text.is_empty() && line.ending() != LineEnding::None).then_some(self.line_height)).unwrap_or_default();
        let (w, h) = self.buffer.lines.iter().fold((0.0f32, 0.0f32), |(mw, mh), line| {
            let (w, h) = line.layout_opt().as_ref().unwrap().iter().fold((0.0f32, 0.0f32), |(w, h), span|
                (w.max(span.w), h+span.line_height_opt.unwrap_or(span.max_ascent+span.max_descent))
            );
            (mw.max(w), mh+h.max(self.line_height))
        });
        (w, h+newline)
    }

    fn update_buffer(&mut self, font_system: &mut impl AsMut<FontSystem>, z: usize) {
        //TODO: check if text changed first
        self.buffer.set_metrics(font_system.as_mut(), Metrics::new(self.size, self.line_height));
        self.buffer.set_wrap(font_system.as_mut(), Wrap::WordOrGlyph);
        self.buffer.set_size(font_system.as_mut(), self.width.map(|w| 1.0+w), None);
        self.buffer.set_text(font_system.as_mut(), &self.text, self.font.1.metadata(z), Shaping::Basic);
    }
}

pub type Font = Arc<(ID, Attrs<'static>)>;

pub struct FontAtlas{
    fonts: Option<HashMap<Arc<Vec<u8>>, Font>>,
    font_system: FontSystem
}

impl AsMut<FontSystem> for FontAtlas {
    fn as_mut(&mut self) -> &mut FontSystem {&mut self.font_system}
}

impl FontAtlas {
    pub fn add(&mut self, raw_font: &[u8]) -> Font {
        let raw_font = Arc::new(raw_font.to_vec());
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
        let to_remove = self.fonts.as_ref().unwrap().iter().filter(|&(_, v)| (Arc::strong_count(v) > 1)).map(|(k, _)| k.clone()).collect::<Vec<_>>();
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
        mut text_areas: Vec<(u16, Area, Text)>
    ) {
        font_atlas.trim();
        self.text_atlas.trim();
        self.viewport.update(queue, Resolution{width: width as u32, height: height as u32});

        text_areas.iter_mut().for_each(|(z, _, t)| {
            t.update_buffer(font_atlas, *z as usize);
        });
        let text_areas = text_areas.iter_mut().map(|(_, a, t)| {
            let default_color = glyphon::Color::rgba(t.color.0, t.color.1, t.color.2, t.color.3);
            let bounds = a.bounds(width, height);
            glyphon::TextArea{
                buffer: &t.buffer,
                left: a.0.0,
                top: a.0.1,
                scale: 1.0,
                bounds: TextBounds {//Sisscor Rect
                    left: bounds.0 as i32,
                    top: bounds.1 as i32,
                    right: (bounds.0 + bounds.2).ceil() as i32,
                    bottom: (bounds.1 + bounds.3).ceil() as i32+2,//TODO: Find out why this is +2
                },
                default_color,
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
