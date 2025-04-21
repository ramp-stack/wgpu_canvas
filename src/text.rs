use glyphon::{Resolution, SwashCache, FontSystem, TextBounds, TextAtlas, Viewport, Metrics, Shaping, Buffer, Family, Cache, Attrs, Wrap};
use wgpu::{DepthStencilState, MultisampleState, TextureFormat, RenderPass, Device, Queue};
use glyphon::fontdb::{Database, Source, ID};
//use glyphon::cosmic_text::LineEnding;

use std::sync::Arc;
use std::collections::HashMap;

use super::{Area, Color};

#[derive(Clone, Debug)]
pub struct Text(Buffer, Color, Font);

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
        Text(buffer, color, font)
    }

    pub fn set_text(&mut self, font_system: &mut impl AsMut<FontSystem>, text: &str) {
        self.0.set_text(font_system.as_mut(), text, self.2.1, Shaping::Basic);
    }

    pub fn get_size(&self) -> (f32, f32) {
        //TODO: no access to text or line_height
        //let newline = self.0.lines.last().and_then(|line| (!text.text.is_empty() && line.ending() != LineEnding::None).then_some(text.line_height)).unwrap_or_default();
        let newline = 0.0;
        let line_height = 0.0;
        let (w, h) = self.0.lines.iter().fold((0.0f32, 0.0f32), |(mw, mh), line| {
            let (w, h) = line.layout_opt().as_ref().unwrap().iter().fold((0.0f32, 0.0f32), |(w, h), span|
                (w.max(span.w), h+span.line_height_opt.unwrap_or(span.max_ascent+span.max_descent))
            );
            (mw.max(w), mh+h.max(line_height))
        });
        (w, h+newline)
    }

    pub fn get_color(&self) -> &Color {&self.1}
    pub fn set_color(&mut self, color: Color) {self.1 = color}

    fn set_z_index(&mut self, z_index: usize) {
        self.0.lines.iter_mut().for_each(|l| l.set_metadata(z_index));
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

        let text_areas = text_areas.iter_mut().map(|(z, a, t)| {
            t.set_z_index(*z as usize);
            let bounds = a.bounds(width, height);
            glyphon::TextArea{
                buffer: &t.0,
                left: a.0.0,
                top: a.0.1,
                scale: 1.0,
                bounds: TextBounds {//Sisscor Rect
                    left: bounds.0 as i32,
                    top: bounds.1 as i32,
                    right: (bounds.0 + bounds.2).ceil() as i32,
                    bottom: (bounds.1 + bounds.3).ceil() as i32+2,//TODO: Find out why this is +2
                },
                default_color: glyphon::Color::rgba(t.1.0, t.1.1, t.1.2, t.1.3),
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
