use super::Color;
use std::ops::Deref;
use std::sync::{Mutex, Arc};
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hasher, Hash};
use lazy_static::lazy_static;
use unicode_segmentation::UnicodeSegmentation;

lazy_static! {
    static ref TEXT_LINES: Arc<Mutex<HashMap<u64, Vec<Line>>>> = Arc::default();

    static ref EMOJI_FONT: Arc<Font> = Arc::new(
        Font::from_bytes(include_bytes!("../emoji.ttf"))
            .expect("failed to load emoji font")
    );
}


fn is_emoji_grapheme(g: &str) -> bool {
    g.chars().any(|c| {
        matches!(
            c as u32,
            0x1F000..=0x1FAFF |
            0x2600..=0x27BF |
            0x2300..=0x23FF
        )
    })
}

#[derive(Debug, Clone)]
pub struct Font(pub fontdue::Font);

impl Font {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        Ok(Font(fontdue::Font::from_bytes(
            bytes,
            fontdue::FontSettings {
                scale: 160.0,
                ..Default::default()
            },
        )?))
    }
}

impl Deref for Font {
    type Target = fontdue::Font;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialEq for Font {
    fn eq(&self, other: &Font) -> bool {
        self.file_hash() == other.file_hash()
    }
}

impl Hash for Font {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.file_hash().hash(hasher)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Hash)]
pub enum Align {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub text: String,
    pub font_size: f32,
    pub line_height: Option<f32>,
    pub font: Arc<Font>,
    pub color: Color,
    pub kerning: f32,
}

impl Hash for Span {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.text.hash(hasher);
        self.font_size.to_bits().hash(hasher);

        if let Some(l) = self.line_height {
            l.to_bits().hash(hasher);
        }

        self.font.hash(hasher);
        self.color.hash(hasher);
        self.kerning.to_bits().hash(hasher);
    }
}

impl Span {
    pub fn new(
        text: String,
        font_size: f32,
        line_height: Option<f32>,
        font: Arc<Font>,
        color: Color,
        kerning: f32,
    ) -> Self {
        Span {
            text,
            font_size,
            line_height,
            font,
            color,
            kerning,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Text {
    pub spans: Vec<Span>,
    pub width: Option<f32>,
    pub align: Align,
    pub cursor: Option<usize>,
    pub max_lines: Option<u32>,
}

impl Hash for Text {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.spans.hash(hasher);

        if let Some(w) = self.width {
            w.to_bits().hash(hasher);
        }

        self.align.hash(hasher);

        if let Some(c) = self.cursor {
            c.hash(hasher);
        }

        if let Some(max) = self.max_lines {
            max.hash(hasher);
        }
    }
}

impl Text {
    pub fn new(
        spans: Vec<Span>,
        width: Option<f32>,
        align: Align,
        max_lines: Option<u32>,
    ) -> Self {
        Text {
            spans,
            width,
            align,
            cursor: None,
            max_lines,
        }
    }

    pub fn size(&self) -> (f32, f32) {
        self.lines()
            .iter()
            .fold((0.0_f32, 0.0_f32), |(w, h), line| {
                (w.max(line.0), h + line.1)
            })
    }

    pub fn cursor_position(&self) -> (f32, f32) {
        let ls = &self.lines();
        let mut ci = 0;

        let mut lines = ls.iter().enumerate().flat_map(|(i, line)| {
            let mut result = Vec::new();

            line.2.iter().for_each(|ch| {
                if self.cursor.unwrap() == ci {
                    result.push((ch.1 .0, i as f32 * ch.4));
                }

                ci += 1;
            });

            if self.cursor.unwrap() == ci {
                result.push((line.0, i as f32 * line.1));
            }

            result
        });

        lines
            .next()
            .or_else(|| {
                ls.last()
                    .map(|l| (l.0, (ls.len().saturating_sub(1) as f32) * l.1))
            })
            .unwrap_or((0.0, 0.0))
    }

    pub fn cursor_click(&mut self, x: f32, y: f32) {
        let mut index = 0;

        for line in self.lines().iter() {
            let line_top = line.2.first().map(|ch| ch.1 .1).unwrap_or(0.0);

            if y >= line_top - 5.0 && y <= line_top + line.1 {
                for (i, ch) in line.2.iter().enumerate() {
                    if x >= ch.1 .0 && x <= ch.1 .0 + ch.5 {
                        self.cursor = Some(if x <= ch.1 .0 + (ch.5 / 2.0) {
                            index + i
                        } else {
                            index + i + 1
                        });

                        return;
                    }
                }

                self.cursor = Some(
                    if x < line.2.first().map(|c| c.1 .0).unwrap_or(0.0) {
                        index
                    } else {
                        index + line.2.len()
                    },
                );

                return;
            }

            index += line.2.len();
        }

        self.cursor = Some(index);
    }

    pub fn len(&self) -> usize {
        self.lines().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub(crate) fn lines(&self) -> Vec<Line> {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);

        TEXT_LINES
            .lock()
            .unwrap()
            .entry(hasher.finish())
            .or_insert_with(|| self.inner_lines())
            .clone()
    }

    pub(crate) fn inner_lines(&self) -> Vec<Line> {
        let mut lines = Vec::new();
        let mut line = Line::default();

        let push = |lines: &mut Vec<Line>, line: &mut Line, lh: f32| {
            if line.2.is_empty() {
                line.1 = line.1.max(lh);
            }

            line.2.iter_mut().for_each(|ch| ch.1 .1 += line.1);
            lines.push(line.take());
        };

        for s in &self.spans {
            let lm = s.font.horizontal_line_metrics(s.font_size).unwrap();
            let lh = s.line_height.unwrap_or(lm.new_line_size);

            for (i, raw) in s.text.split('\n').enumerate() {
                if i > 0 {
                    push(&mut lines, &mut line, lh);
                }

                for word in raw.split_inclusive(char::is_whitespace) {
                    let mut ww = 0.0;

                    let glyphs: Vec<_> = UnicodeSegmentation::graphemes(word, true)
                        .map(|g| {
                            let font = if is_emoji_grapheme(g) {
                                EMOJI_FONT.clone()
                            } else {
                                s.font.clone()
                            };

                            let mut aw = 0.0_f32;
                            let mut xmin = 0.0_f32;
                            let mut ymin = 0.0_f32;
                            let mut xmax = 0.0_f32;
                            let mut ymax = 0.0_f32;
                            let mut first = true;

                            for c in g.chars() {
                                // Skip emoji join/control codepoints for metrics.
                                if c == '\u{fe0f}' || c == '\u{200d}' {
                                    continue;
                                }

                                let m = font.metrics(c, s.font_size);

                                let gxmin = aw + m.bounds.xmin;
                                let gymin = m.bounds.ymin;
                                let gxmax = gxmin + m.bounds.width;
                                let gymax = gymin + m.bounds.height;

                                if first {
                                    xmin = gxmin;
                                    ymin = gymin;
                                    xmax = gxmax;
                                    ymax = gymax;
                                    first = false;
                                } else {
                                    xmin = xmin.min(gxmin);
                                    ymin = ymin.min(gymin);
                                    xmax = xmax.max(gxmax);
                                    ymax = ymax.max(gymax);
                                }

                                aw += m.advance_width;
                            }

                            let w = xmax - xmin;
                            let h = ymax - ymin;

                            ww += aw + s.kerning;

                            (
                                g.to_string(),
                                font,
                                (xmin, ymin, w, h),
                                aw,
                                g.chars().all(char::is_whitespace),
                            )
                        })
                        .collect();

                    if let Some(wmax) = self.width
                        && ww < wmax
                        && line.0 + ww > wmax
                        && !line.2.is_empty()
                    {
                        push(&mut lines, &mut line, lh);
                    }

                    for (g, font, (xmin, ymin, w, h), aw, is_whitespace) in glyphs {
                        if let Some(wmax) = self.width {
                            if line.0 + aw + s.kerning > wmax && !line.2.is_empty() {
                                push(&mut lines, &mut line, lh);
                            }

                            if line.2.is_empty() && is_whitespace {
                                continue;
                            }
                        }

                        let y = lines.iter().fold(0.0, |h, l| h + l.1) - ymin - h + lm.descent;

                        line.2.push(Character(
                            g.clone(),
                            (line.0 + xmin, y, w, h),
                            font,
                            Some(s.color),
                            lh,
                            aw,
                        ));

                        line.0 += aw + s.kerning;
                        line.1 = line.1.max(lh);
                    }
                }
            }
        }

        if !line.2.is_empty() {
            push(&mut lines, &mut line, 0.0);
        }

        lines.iter_mut().for_each(|line| {
            let offset = match self.align {
                Align::Left => 0.0,
                Align::Center => self.width.map_or(0.0, |w| (w - line.0) / 2.0),
                Align::Right => self.width.map_or(0.0, |w| w - line.0),
            };

            line.2.iter_mut().for_each(|ch| ch.1 .0 += offset);
            line.0 += offset;
        });

        if let Some(max) = self.max_lines {
            let len = lines.len();
            lines.truncate(max as usize);

            if len > max as usize {
                let y = lines.iter().fold(0.0, |h, l| h + l.1);

                if let Some(last) = lines.last_mut() {
                    last.2.truncate(last.2.len().saturating_sub(3));

                    let s = &self.spans[0];
                    let lm = s.font.horizontal_line_metrics(s.font_size).unwrap();
                    let lh = s.line_height.unwrap_or(lm.new_line_size);
                    let mut x = last.2.last().map(|g| g.1 .0 + g.5).unwrap_or(0.0);

                    for c in "...".chars() {
                        let m = s.font.metrics(c, s.font_size);
                        let aw = m.advance_width + s.kerning;

                        last.2.push(Character(
                            c.to_string(),
                            (
                                x + m.bounds.xmin,
                                y - m.bounds.ymin - m.bounds.height + lm.descent * s.font_size,
                                m.bounds.width,
                                m.bounds.height,
                            ),
                            s.font.clone(),
                            Some(s.color),
                            lh,
                            aw,
                        ));

                        x += aw;
                        last.0 += aw;
                        last.1 = last.1.max(lh);
                    }
                }
            }
        }

        lines
    }
}

#[derive(Debug, Clone)]
pub struct Character(
    pub String,
    pub (f32, f32, f32, f32),
    pub Arc<Font>,
    pub Option<Color>,
    pub f32,
    pub f32,
);

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