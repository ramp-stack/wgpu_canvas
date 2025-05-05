use glyphon::{Resolution, SwashCache, FontSystem, TextBounds, TextAtlas, Viewport, Metrics, Shaping, Buffer, Family, Cache, Attrs, Wrap, Affinity};
use wgpu::{DepthStencilState, MultisampleState, TextureFormat, RenderPass, Device, Queue};
use glyphon::fontdb::{Database, Source, ID};

use std::sync::Arc;
use std::collections::HashMap;

use super::{Area, Color};

pub use glyphon::cosmic_text::{Align};
use glyphon::cosmic_text::Cursor as CosmicCursor;

#[derive(Debug, Clone, Copy)]
pub struct Cursor {
    pub line: usize,
    pub index: usize,
    pub affinity: Affinity,
}

impl Default for Cursor {
    fn default() -> Self {Cursor { line: 0, index: 0, affinity: Affinity::Before }}
}

impl Cursor {
    fn from(cursor: CosmicCursor) -> Self {
        Cursor { line: cursor.line, index: cursor.index, affinity: cursor.affinity }
    }

    pub fn position(&mut self, buffer: &Buffer) -> Option<(f32, f32)> {
        // println!("Getting position");
       
        buffer.layout_runs().enumerate().find_map(|(i, run)| {
            if i == self.line {
                let mut x_pos = 0.0;
                for glyph in run.glyphs {
                    if glyph.start <= self.index && self.index < glyph.end {
                        return Some((x_pos, run.line_y - run.line_height));
                    }
                    x_pos += glyph.w;
                }
                let line_width: f32 = run.glyphs.iter().map(|g| g.w).sum();
                return Some((line_width, run.line_y - run.line_height));
            }
            None
        })        
    }

    pub fn move_right(&mut self, buffer: &Buffer) {
        let mut runs = buffer.layout_runs().enumerate();
    
        if let Some((_, run)) = runs.find(|(i, _)| *i == self.line) {
            let line_end = run.glyphs.last().map(|g| g.end).unwrap_or(0);
            if self.index < line_end {
                self.index += 1;
            } else if let Some((next_line, next_run)) = runs.find(|(i, _)| *i == self.line + 1) {
                self.line = next_line;
                self.index = next_run.glyphs.last().map(|g| g.end).unwrap_or(0);
            }
        }
    }

    pub fn move_left(&mut self, buffer: &Buffer) {
        println!("MOVE LEFT");
        buffer.layout_runs().enumerate().for_each(|(i, run)| {
            println!("Run: {:?}", i);
            if i == self.line {
                println!("Line found. {:?}", self.line);
                match run.glyphs.len() > 0 {
                    true => {
                        self.index -= 1;
                        if self.index == 0 {
                            self.line -= 1;
                            println!("END OF LINE {:?}", self.line);
                        }
                    },
                    false => {
                        // self.line -= 1;
                        buffer.layout_runs().enumerate().for_each(|(i, prev_run)| {
                            if i == self.line {
                                self.index = prev_run.glyphs.len();
                            }
                        });
                    }
                }
            }
        });
    }
    

    pub fn move_up(&mut self, buffer: &Buffer) {
        let mut runs: Vec<_> = buffer.layout_runs().collect();

        if runs.len() != 1 && self.line != 0 {
            self.line -= 1;
        }
    }

    pub fn move_down(&mut self, buffer: &Buffer) {
        let mut runs: Vec<_> = buffer.layout_runs().collect();

        if runs.len() != (self.line + 1) {
            self.line += 1;
        }
    }

    // pub fn new_line(&mut self, buffer: &Buffer) {
    //     let mut runs = buffer.layout_runs().enumerate();
    
    //     if let Some((_, run)) = runs.find(|(i, _)| *i == self.line) {
    //         let mut remaining_glyphs = 0;
    
    //         for glyph in run.glyphs {
    //             if glyph.start >= self.index {
    //                 remaining_glyphs += 1;  
    //             }
    //         }

    //         let glyphs = run.glyphs.len();
    //         let to_remove = glyphs - remaining_glyphs;
    //         run.glyphs.retain(|(i, _)| i >= to_remove);

    //         // let new_glyphs = run.glyphs.iter().filter(|glyph| glyph.end <= self.index).cloned().collect();
    
    //         // run.glyphs = new_glyphs;
    
    //         // let mut new_run = run.clone(); 
    //         // new_run.glyphs = remaining_glyphs;
    
    //         // buffer.layout_runs().insert(self.line + 1, new_run);
    
    //         // self.line += 1;
    //         // self.index = 0;
    //     }
    // }
    
}

#[derive(Debug, Clone)]
pub struct Span{
    pub text: String, 
    pub font_size: f32,
    pub line_height: f32,
    pub font: Font,
    pub color: Color
}

impl Span {
    pub fn new(text: &str, font_size: f32, line_height: f32, font: Font, color: Color) -> Self {
        Span{text: text.to_string(), font_size, line_height, font, color}
    }
    pub fn into_inner(&self, z_index: usize) -> (&str, Attrs<'static>) {
        let color = glyphon::cosmic_text::Color::rgba(self.color.0, self.color.1, self.color.2, self.color.3);
        let attrs = self.font.1.clone().color(color).metadata(z_index).metrics(Metrics::new(self.font_size, self.line_height));
        (&self.text, attrs)
    }
}

#[derive(Debug, Clone)]
pub struct Text{
    pub spans: Vec<Span>,
    pub width: Option<f32>,
    pub align: Align,
    pub cursor: Option<Cursor>,
}

impl Text {
    pub fn new(spans: Vec<Span>, width: Option<f32>, align: Align, with_cursor: bool) -> Self {
        Text{spans, width, align, cursor: with_cursor.then(|| Cursor::default())}
    }

    pub fn set_cursor(&mut self, font_system: &mut impl AsMut<FontAtlas>, x: f32, y: f32) {
        let buffer = self.get_buffer(font_system.as_mut(), 0);
        self.cursor = buffer.hit(x, y).map(Cursor::from);
    }

    pub fn cursor_position(&mut self, font_system: &mut impl AsMut<FontAtlas>) -> Option<(f32, f32)> {
        let buffer = self.get_buffer(font_system.as_mut(), 0);
        self.cursor.as_mut().map(|c| c.position(&buffer))?
    }

    pub fn cursor_right(&mut self, font_system: &mut impl AsMut<FontAtlas>) {
        let buffer = self.get_buffer(font_system.as_mut(), 0);
        self.cursor.as_mut().map(|c| c.move_right(&buffer));
    }

    pub fn cursor_left(&mut self, font_system: &mut impl AsMut<FontAtlas>) {
        let buffer = self.get_buffer(font_system.as_mut(), 0);
        self.cursor.as_mut().map(|c| c.move_left(&buffer));
    }

    pub fn cursor_up(&mut self, font_system: &mut impl AsMut<FontAtlas>) {
        let buffer = self.get_buffer(font_system.as_mut(), 0);
        self.cursor.as_mut().map(|c| c.move_up(&buffer));
    }

    pub fn cursor_down(&mut self, font_system: &mut impl AsMut<FontAtlas>) {
        let buffer = self.get_buffer(font_system.as_mut(), 0);
        // self.cursor.as_mut().map(|c| c.new_line(&buffer));
    }

    pub fn size(&self, font_system: &mut impl AsMut<FontAtlas>) -> (f32, f32) {
       Self::buffer_size(&self.get_buffer(font_system.as_mut(), 0), &self.spans)
    }

    pub fn set_color(&mut self, color: Color) {
        self.spans.iter_mut().for_each(|s| s.color = color);
    }

    pub fn width(mut self, width: Option<f32>) -> Self {self.width = width; self}

    fn get_buffer(&self, font_system: &mut impl AsMut<FontSystem>, z_index: usize) -> Buffer {
        let default_attrs = self.spans.first().expect("Text must have at least one span even if its empty").into_inner(0).1;
        let metrics = Metrics::from(default_attrs.metrics_opt.unwrap());
        let mut buffer = Buffer::new(font_system.as_mut(), metrics);
        buffer.set_wrap(font_system.as_mut(), Wrap::WordOrGlyph);
        buffer.set_size(font_system.as_mut(), self.width.map(|w| 1.0+w), None);
        buffer.set_rich_text(
            font_system.as_mut(), self.spans.iter().map(|s| s.into_inner(z_index)), 
            &default_attrs, Shaping::Advanced, Some(self.align)
        );
        buffer
    }

    fn buffer_size(buffer: &Buffer, spans: &[Span]) -> (f32, f32) {
        let new_line = spans.iter().rev().find_map(|s| (!s.text.is_empty()).then(||
            (s.text.get(s.text.len()-1..) == Some("\n")).then_some(s.line_height)
        )).flatten().unwrap_or_default();
        
        let (w, h) = buffer.layout_runs().fold((0.0f32, 0.0f32), |(max_w, total_h), run| {
            let w = run.line_w;
            let h = run.line_height;
            (max_w.max(w), total_h + h)
        });

        (w, h+new_line)
    }
}

pub type Font = Arc<(ID, Attrs<'static>)>;

pub struct FontAtlas{
    fonts: Option<HashMap<Arc<Vec<u8>>, Font>>,
    font_system: FontSystem
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

impl AsMut<FontSystem> for FontAtlas {
    fn as_mut(&mut self) -> &mut FontSystem {&mut self.font_system}
}

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
        text_areas: Vec<(u16, Area, Text)>
    ) {
        font_atlas.trim();
        self.text_atlas.trim();
        self.viewport.update(queue, Resolution{width: width as u32, height: height as u32});
        let text_areas = text_areas.into_iter().map(|(z, a, t)| {
            let mut b = t.get_buffer(font_atlas, z as usize);
            let width = Text::buffer_size(&b, &t.spans).0;
            b.set_size(&mut font_atlas.font_system, Some(width), None);
            (a, b)
        }).collect::<Vec<_>>();
        let text_areas = text_areas.iter().map(|(a, b)| {
            let bounds = a.bounds(width, height);
            glyphon::TextArea{
                buffer: b,
                left: a.0.0,
                top: a.0.1,
                scale: 1.0,
                bounds: TextBounds {//Sisscor Rect
                    left: bounds.0 as i32,
                    top: bounds.1 as i32,
                    right: (bounds.0 + bounds.2).ceil() as i32,
                    bottom: (bounds.1 + bounds.3).ceil() as i32,
                },
                default_color: glyphon::Color::rgba(139, 0, 139, 255),
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
