use wgpu::{
    DepthStencilState,
    MultisampleState,
    TextureFormat,
    RenderPass,
    Device,
    Queue,
};

use glyphon::{
    Resolution,
    SwashCache,
    FontSystem,
    TextBounds,
    TextAtlas,
    TextArea,
    Viewport,
    Metrics,
    Shaping,
    Buffer,
    Family,
    Color,
    Cache,
    Attrs,
    Wrap
};

use glyphon::fontdb::{Database, Source};

use std::sync::Arc;
use std::collections::HashMap;

pub struct Text<'a> {
    pub x: u32,
    pub y: u32,
    pub w: Option<u32>,
    pub text: &'a str,
    pub color: &'a str,
    pub font: &'a [u8],
    pub size: f32,
    pub line_height: f32,
    pub z_index: u32,
    pub bounds: (u32, u32, u32, u32)
}

impl<'a> Text<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        x: u32,
        y: u32,
        w: Option<u32>,
        text: &'a str,
        color: &'a str,
        font: &'a [u8],
        size: f32,
        line_height: f32,
        z_index: u32,
        bounds: (u32, u32, u32, u32)
    ) -> Self {
        Text{x, y, w, text, color, font, size, line_height, z_index, bounds}
    }

    fn color(&self) -> Color {
        let ce = "Color was not a Hex Value";
        let hex: [u8; 3] = hex::decode(self.color).expect(ce).try_into().expect(ce);
        Color::rgb(hex[0], hex[1], hex[2])
    }
}

pub struct TextRenderer {
    text_renderer: glyphon::TextRenderer,
    font_system: FontSystem,
    swash_cache: SwashCache,
    text_atlas: TextAtlas,
    viewport: Viewport,
    fonts: HashMap<Vec<u8>, Attrs<'static>>
}

impl TextRenderer {
    pub fn new(
        queue: &Queue,
        device: &Device,
        texture_format: &TextureFormat,
        multisample: MultisampleState,
        depth_stencil: Option<DepthStencilState>,
    ) -> Self {
        let text_cache = Cache::new(device);
        let mut text_atlas = TextAtlas::new(device, queue, &text_cache, *texture_format);
        let font_system = FontSystem::new_with_locale_and_db("".to_string(), Database::new());
        let text_renderer = glyphon::TextRenderer::new(&mut text_atlas, device, multisample, depth_stencil);

        TextRenderer{
            text_renderer,
            font_system,
            text_atlas,
            viewport: Viewport::new(device, &text_cache),
            swash_cache: SwashCache::new(),
            fonts: HashMap::new()
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn prepare(
        &mut self,
        device: &Device,
        queue: &Queue,
        physical_width: u32,
        physical_height: u32,
        text: Vec<Text>
    ) {
        self.text_atlas.trim();
        self.viewport.update(
            queue,
            Resolution {
                width: physical_width,
                height: physical_height,
            },
        );

        let buffers = text.iter().map(|t| {
            let font = self.get_font(t.font).metadata(t.z_index as usize);
            let mut buffer = Buffer::new(&mut self.font_system, Metrics::new(t.size, t.line_height));
            buffer.set_wrap(&mut self.font_system, Wrap::WordOrGlyph);
            buffer.set_size(&mut self.font_system, t.w.map(|w| w as f32), None);
            buffer.set_text(
                &mut self.font_system,
                t.text,
                font,
                Shaping::Advanced
            );
            buffer
        }).collect::<Vec<_>>();

        let text_areas = text.into_iter().zip(buffers.iter()).map(|(t, b)| {
            let left = t.bounds.0 as i32;
            let top = t.bounds.1 as i32;
            TextArea {
                buffer: b,
                left: t.x as f32,
                top: t.y as f32,
                scale: 1.0,
                bounds: TextBounds {//Sisscor Rect
                    left,
                    top,
                    right: left + t.bounds.2 as i32,
                    bottom: top + t.bounds.3 as i32,
                },
                default_color: t.color(),
                custom_glyphs: &[]
            }
        });

        self.text_renderer.prepare_with_depth(
            device,
            queue,
            &mut self.font_system,
            &mut self.text_atlas,
            &self.viewport,
            text_areas,
            &mut self.swash_cache,
            |z: usize| ((z as u32) as f32) / u32::MAX as f32
        ).unwrap();
    }

    pub fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        self.text_renderer.render(&self.text_atlas, &self.viewport, render_pass).unwrap();
    }

    pub fn messure_text(&mut self, t: &Text) -> (u32, u32) {
        let font = self.get_font(t.font);
        let mut buffer = Buffer::new(&mut self.font_system, Metrics::new(t.size, t.line_height));
        buffer.set_wrap(&mut self.font_system, Wrap::WordOrGlyph);
        buffer.set_size(&mut self.font_system, t.w.map(|w| w as f32), None);
        buffer.set_text(
            &mut self.font_system,
            t.text,
            font,
            Shaping::Advanced
        );
        buffer.lines.first().map(|line|
            line.layout_opt().as_ref().unwrap().iter().fold((0, 0), |(w, h), span| {
                (w.max(span.w as u32), h+span.line_height_opt.unwrap_or(span.max_ascent+span.max_descent) as u32)
            })
        ).unwrap_or((0, 0))
    }

    fn get_font(&mut self, font: &[u8]) -> Attrs<'static> {
        match self.fonts.get(font) {
            Some(font_a) => *font_a,
            None => {
                let font = font.to_vec();
                let database = self.font_system.db_mut();
                let id = database.load_font_source(Source::Binary(Arc::new(font.clone())))[0];
                let face = database.face(id).unwrap();
                let attrs = Attrs::new()
                    .family(Family::<'static>::Name(face.families[0].0.clone().leak()))
                    .stretch(face.stretch)
                    .style(face.style)
                    .weight(face.weight);
                self.fonts.insert(font, attrs);
                attrs
            }
        }
    }
}
