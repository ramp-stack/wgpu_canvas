use glyphon::cosmic_text::{Action, Buffer, Motion};
use glyphon::{Resolution, SwashCache, TextRenderer, TextBounds, TextAtlas, TextArea, FontSystem, Metrics, Attrs};
use wgpu::{Device, Queue};

use super::Area;

pub struct TextEditor {
    pub buffer: Buffer,
    pub swash_cache: SwashCache,
    pub scale: f32,
}

impl TextEditor {
    pub fn new(
        font_system: &mut FontSystem,
        metrics: Metrics,
        scale: f32,
        _attrs: Attrs, // Only needed if you're setting initial text
    ) -> Self {
        let swash_cache = SwashCache::new();
        let buffer = Buffer::new(font_system, metrics);

        Self {
            buffer,
            swash_cache,
            scale,
        }
    }

    pub fn set_metrics(&mut self, metrics: Metrics) {
        self.buffer.set_metrics(metrics);
    }

    pub fn move_cursor(&mut self, motion: Motion, selecting: bool) {
        self.buffer.action(Action::Motion { motion, modify: selecting });
    }

    pub fn handle_input(&mut self, input: &str) {
        for c in input.chars() {
            self.buffer.action(Action::Insert(c));
        }
    }

    pub fn click(&mut self, x: f32, y: f32, selecting: bool) {
        self.buffer.action(Action::Click { x, y, select: selecting });
    }

    pub fn drag(&mut self, x: f32, y: f32) {
        self.buffer.action(Action::Drag { x, y });
    }

    pub fn scroll(&mut self, lines: f32) {
        self.buffer.action(Action::Scroll { lines });
    }

    pub fn render(
        &mut self,
        device: &Device,
        queue: &Queue,
        width: f32,
        height: f32,
        font_atlas: &mut super::FontAtlas,
        text_renderer: &mut TextRenderer,
        area: Area,
    ) {
        font_atlas.trim();
        text_renderer.text_atlas.trim();
        text_renderer.viewport.update(queue, Resolution {
            width: width as u32,
            height: height as u32,
        });

        let bounds = TextBounds {
            left: area.bounds.0 as i32,
            top: area.bounds.1 as i32,
            right: (area.bounds.0 + area.bounds.2).ceil() as i32,
            bottom: (area.bounds.1 + area.bounds.3).ceil() as i32 + 2,
        };

        let text_area = TextArea {
            buffer: &self.buffer,
            left: area.offset.0,
            top: area.offset.1,
            scale: self.scale,
            bounds,
            default_color: glyphon::Color::rgb(1.0, 1.0, 1.0),
            custom_glyphs: &[],
        };

        text_renderer.text_renderer.prepare_with_depth(
            device,
            queue,
            &mut font_atlas.font_system,
            &mut text_renderer.text_atlas,
            &text_renderer.viewport,
            [text_area].into_iter(),
            &mut self.swash_cache,
            |z: usize| ((z as u16) as f32) / u16::MAX as f32,
        ).unwrap();
    }
}
