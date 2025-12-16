use super::Color;
use std::ops::Deref;
use std::sync::Arc;

pub type Cursor = usize;

#[derive(Debug, Clone)]
pub struct Font(pub fontdue::Font);
impl Font {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        Ok(Font(fontdue::Font::from_bytes(bytes, fontdue::FontSettings{scale: 160.0, ..Default::default()})?))
    }
}

impl Deref for Font {
    type Target = fontdue::Font;
    fn deref(&self) -> &Self::Target {&self.0}
}

impl PartialEq for Font {
    fn eq(&self, other: &Font) -> bool {
        self.file_hash() == other.file_hash()
    }
}

/// Text alignment enumerator.
#[derive(Debug, Clone, Copy, PartialEq, Hash)]
pub enum Align {
    Left, 
    Center, 
    Right
}

#[derive(Debug, Clone, PartialEq)]
pub struct Span{
    /// The text content.  
    pub text: String, 
    /// Size of the font in logical pixels. 
    pub font_size: f32,
    /// Optional custom line height.
    pub line_height: Option<f32>,
    /// The font face used for rendering.  
    pub font: Arc<Font>,
    /// The text color.
    pub color: Color,
    /// Additional spacing between characters.
    pub kerning: f32,
}

impl Span {
    pub fn new(text: String, font_size: f32, line_height: Option<f32>, font: Arc<Font>, color: Color, kerning: f32) -> Self {
        Span{text, font_size, line_height, font, color, kerning}
    }
}

/// # Text
///
/// A text container composed of one or more [`Spans`](Span).  
/// Supports layout, alignment, scaling, and cursor placement for editing or interaction.
#[derive(Debug, Clone, PartialEq)]
pub struct Text {
    /// A vector of styled [`Span`] segments that make up the text content.
    pub spans: Vec<Span>,
    /// Optional maximum width.  
    pub width: Option<f32>,
    /// Horizontal alignment of the text.
    pub align: Align,
    /// Optional cursor position for editable or interactive text.
    pub cursor: Option<Cursor>,
    /// Optional maximum number of rendered lines.
    pub max_lines: Option<u32>,
}

impl Text {
    pub fn new(spans: Vec<Span>, width: Option<f32>, align: Align, max_lines: Option<u32>) -> Self {
        Text{spans, width, align, cursor: None, max_lines}
    }

    pub fn size(&self) -> (f32, f32) {
        self.lines().iter().fold((0.0, 0.0), |(w, h), line| (w.max(line.0), h+line.1))
    }

    pub fn cursor_position(&self) -> (f32, f32) {
        let ls = &self.lines();
        let mut ci = 0;

        let mut lines = ls.iter().enumerate().flat_map(|(i, line)| {
            let mut result = Vec::new();
            line.2.iter().for_each(|ch| {
                if self.cursor.unwrap() == ci { result.push((ch.1.0, i as f32 * ch.4)); }
                ci += 1;
            });

            if self.cursor.unwrap() == ci { result.push((line.0, i as f32 * line.1)); }

            result
        });
        
        lines.next().or_else(|| ls.last().map(|l| (l.0, (ls.len().saturating_sub(1) as f32) * l.1))).unwrap_or((0.0, 0.0))
    }

    pub fn cursor_click(&mut self, x: f32, y: f32) {
        let mut index = 0;

        for line in self.lines().iter() {
            let line_top = line.2.first().map(|ch| ch.1.1).unwrap_or(0.0);

            if y >= line_top - 5.0 && y <= line_top + line.1 {
                for (i, ch) in line.2.iter().enumerate() {
                    if x >= ch.1.0 && x <= ch.1.0 + ch.5 {
                        match x <= ch.1.0 + (ch.5 / 2.0) {
                            true => self.cursor = Some(index + i),
                            false => self.cursor = Some(index + i + 1)
                        }
                        return;
                    }
                }

                match x < line.2.first().map(|c| c.1.0).unwrap_or(0.0) {
                    true => self.cursor = Some(index),
                    false => self.cursor = Some(index + line.2.len())
                }

                return;
            }

            index += line.2.len();
        }

        self.cursor = Some(index);
    }

    pub(crate) fn lines(&self) -> Vec<Line> {
        let mut lines = Vec::new();
        let mut current_line = Line::default();
        self.spans.iter().for_each(|s| {
            let lm = s.font.horizontal_line_metrics(s.font_size).unwrap();
            let lh = s.line_height.unwrap_or(lm.new_line_size);
            s.text.split('\n').for_each(|raw_line| {
                raw_line.split_inclusive(|c: char| c.is_whitespace()).for_each(|word| {
                    let mut word_width = 0.0;
                    let glyphs: Vec<_> = word.chars().map(|c| {
                        let m = s.font.metrics(c, s.font_size);
                        let aw = m.advance_width;
                        word_width += aw + s.kerning;
                        (c, (m.bounds.xmin, m.bounds.ymin, m.bounds.width, m.bounds.height), aw)
                    }).collect();

                    if let Some(width) = self.width {
                        if word_width <= width || self.max_lines.is_none() && current_line.0 + word_width > width && !current_line.2.is_empty() {
                            current_line.2.iter_mut().for_each(|ch| ch.1.1 += current_line.1);
                            lines.push(current_line.take());
                        }
                        for (c, (xmin, ymin, w, h), aw) in glyphs.iter() {
                            if current_line.0 + aw > width && !current_line.2.is_empty() {
                                current_line.2.iter_mut().for_each(|ch| ch.1.1 += current_line.1);
                                lines.push(current_line.take());
                            }
                            let y = lines.iter().fold(0.0, |h, l| h + l.1) - ymin - h + lm.descent;
                            current_line.2.push(Character(*c, (current_line.0 + xmin, y, *w, *h),
                                s.font.clone(), s.color, lh, *aw,
                            ));
                            current_line.0 += aw + s.kerning;
                            current_line.1 = current_line.1.max(lh);
                        }
                    } else {
                        for (c, (xmin, ymin, w, h), aw) in glyphs.iter() {
                            let y = lines.iter().fold(0.0, |h, l| h + l.1) - ymin - h + lm.descent;
                            current_line.2.push(Character(*c, (current_line.0 + xmin, y, *w, *h),
                                s.font.clone(), s.color, lh, *aw,
                            ));
                            current_line.0 += aw + s.kerning;
                            current_line.1 = current_line.1.max(lh);
                        }
                    }

                });
                if current_line.2.is_empty() { current_line.1 = current_line.1.max(lh); }
                current_line.2.iter_mut().for_each(|ch| ch.1.1 += current_line.1);
                lines.push(current_line.take());
            })
        });

        // last line handled
        if !current_line.2.is_empty() {
            current_line.2.iter_mut().for_each(|ch| ch.1.1 += current_line.1);
            lines.push(current_line.take());
        }

        // alignment
        lines.iter_mut().for_each(|line| {
            let offset_x = match self.align {
                Align::Left => 0.0,
                Align::Center => self.width.map_or(0.0, |w| (w - line.0) / 2.0),
                Align::Right => self.width.map_or(0.0, |w| w - line.0),
            };
            line.2.iter_mut().for_each(|ch| ch.1.0 += offset_x);
            line.0 += offset_x;
        });

        // line max
        match self.max_lines {
            None => lines,
            Some(max) => {
                let len = lines.len();
                let mut lines: Vec<_> = lines.into_iter().enumerate().filter_map(|(i, line)| (i < max as usize).then_some(line)).collect();
                let y = lines.iter().fold(0.0, |h, l| h + l.1);
                if len > max as usize {
                    if let Some(last) = lines.last_mut() {
                        last.2.truncate(last.2.len().saturating_sub(3));
                        let s = &self.spans[0];
                        let lm = s.font.horizontal_line_metrics(s.font_size).unwrap();
                        let lh = s.line_height.unwrap_or(lm.new_line_size);

                        let glyphs: Vec<_> = "...".chars().map(|c| {
                            let m = s.font.metrics(c, s.font_size);
                            let aw = m.advance_width + s.kerning;
                            (c, (m.bounds.xmin, m.bounds.ymin, m.bounds.width, m.bounds.height), aw)
                        }).collect();

                        let mut start_x = last.2.last().map(|g| g.1.0 + g.5).unwrap_or(0.0);
                        for (c, (xmin, ymin, w, h), aw) in glyphs.iter() {
                            last.2.push(Character(*c, (start_x + xmin, y - ymin - h + lm.descent * s.font_size, *w, *h),
                                s.font.clone(), s.color, lh, *aw,
                            ));
                            start_x += *aw;
                            last.0 += *aw;
                            last.1 = last.1.max(lh);
                        }
                    }
                }

                lines
            }
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Character(pub char, pub (f32, f32, f32, f32), pub Arc<Font>, pub Color, pub f32, pub f32);

#[derive(Debug, Clone, Default)]
pub(crate) struct Line(pub f32, pub f32, pub Vec<Character>);
impl Line {
    fn take(&mut self) -> Self {
        let l = Line(self.0, self.1, self.2.drain(..).collect());
        self.0 = 0.0;
        self.1 = 0.0;
        l
    }
}

