use super::Color;
use std::ops::Deref;
use std::sync::{Mutex, Arc};
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hasher, Hash};
use lazy_static::lazy_static;
use unicode_segmentation::UnicodeSegmentation;

lazy_static! {
    static ref TEXT_LINES: Arc<Mutex<HashMap<u64, Vec<Line>>>> = Arc::default();

    pub(crate) static ref EMOJI_FONT: Font = Font::from_bytes(include_bytes!("../emoji.ttf"))
        .expect("failed to load emoji font");
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
#[derive(Debug)]
pub struct FontInner {
    fontdue: fontdue::Font,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Font(pub Arc<FontInner>);

/// Bounding box + advance for an emoji grapheme, computed via swash since
/// fontdue can't read glyph outlines out of a color (COLR/CBDT) font.
pub(crate) struct SwashMetrics {
    pub xmin: f32,
    pub ymin: f32,
    pub width: f32,
    pub height: f32,
    pub advance_width: f32,
}

impl Font {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        let fontdue_font = fontdue::Font::from_bytes(
            bytes,
            fontdue::FontSettings {
                scale: 160.0,
                ..Default::default()
            },
        )?;

        Ok(Font(Arc::new(FontInner {
            fontdue: fontdue_font,
            bytes: bytes.to_vec(),
        })))
    }

    /// Builds a swash `FontRef` borrowing this font's original byte buffer.
    /// Used for emoji rendering via swash, as opposed to the fontdue path
    /// used for regular glyph rasterization.
    pub(crate) fn swash_font(&self) -> swash::FontRef<'_> {
        swash::FontRef::from_index(&self.0.bytes, 0)
            .expect("invalid font data")
    }

    /// Shapes and measures a single grapheme (e.g. one emoji, possibly a
    /// ZWJ sequence) using swash, returning a bounding box comparable to
    /// fontdue's `OutlineBounds` plus the shaped advance width. fontdue
    /// can't be used for this because color emoji glyphs typically have no
    /// `glyf` outline data for it to measure -- only COLR/CPAL or embedded
    /// bitmap tables, which fontdue doesn't parse.
    pub(crate) fn swash_metrics(&self, grapheme: &str, size: f32) -> Option<SwashMetrics> {
        use swash::scale::{Render, ScaleContext, Source, StrikeWith};
        use swash::shape::ShapeContext;

        let mut scale_context = ScaleContext::new();

        let mut scaler = scale_context
            .builder(self.swash_font())
            .size(size)
            .hint(false)
            .build();

        let mut shape_context = ShapeContext::new();

        let mut shaper = shape_context
            .builder(self.swash_font())
            .size(size)
            .build();

        shaper.add_str(grapheme);

        let mut glyph_id = None;
        let mut advance_width = 0.0_f32;

        shaper.shape_with(|cluster| {
            for glyph in cluster.glyphs {
                if glyph_id.is_none() {
                    glyph_id = Some(glyph.id);
                }

                advance_width += glyph.advance;
            }
        });

        let glyph_id = glyph_id?;

        let mut render = Render::new(&[
            Source::ColorBitmap(StrikeWith::BestFit),
            Source::ColorOutline(0),
            Source::Outline,
        ]);

        let image = render.render(&mut scaler, glyph_id)?;

        if image.placement.width == 0 || image.placement.height == 0 {
            return None;
        }

        let height = image.placement.height as f32;
        let ymin = image.placement.top as f32 - height;

        Some(SwashMetrics {
            xmin: image.placement.left as f32,
            ymin,
            width: image.placement.width as f32,
            height,
            advance_width,
        })
    }
}

impl Deref for Font {
    type Target = fontdue::Font;

    fn deref(&self) -> &Self::Target {
        &self.0.fontdue
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
    pub font: Font,
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
        font: Font,
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
    /// Byte offset into the text of all spans joined together. Always sits on
    /// a grapheme boundary when set by `cursor_click`; other values are
    /// clamped when drawn.
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
        text: &str,
        font: Font,
        font_size: f32,
        color: Color,
        align: Align,
    ) -> Self {
        Text {
            spans: vec![Span::new(text.to_string(), font_size, None, font, color, 0.0)],
            width: None,
            align,
            cursor: None,
            max_lines: None,
        }
    }

    pub fn scale(&mut self, scale: f32) {
        self.width = self.width.map(|w| scale*w);
        self.spans.iter_mut().for_each(|span| {
            span.font_size *= scale;
            span.line_height = span.line_height.map(|l| scale*l);
            span.kerning *= scale;
        });
    }

    /// Width and height of the laid-out text. When the text ends with a
    /// newline, the height includes the empty line below it, where the caret
    /// sits (see `locate`), so whatever holds this text is tall enough for it.
    pub fn size(&self) -> (f32, f32) {
        let lines = self.lines();
        let (w, h) = lines
            .iter()
            .fold((0.0_f32, 0.0_f32), |(w, h), line| {
                (w.max(line.0), h + line.1)
            });

        let fits = self.max_lines.is_none_or(|max| lines.len() < max as usize);
        if self.ends_with_newline() && fits {
            (w, h + self.trailing_line_height())
        } else {
            (w, h)
        }
    }

    /// Height of the empty line after a trailing newline: the line height of
    /// the last non-empty span.
    fn trailing_line_height(&self) -> f32 {
        self.spans.iter().rev().find(|s| !s.text.is_empty()).map_or(0.0, |s| {
            s.line_height.unwrap_or_else(|| {
                s.font.horizontal_line_metrics(s.font_size).map_or(0.0, |m| m.new_line_size)
            })
        })
    }

    /// Top-left of the caret for `self.cursor`: x is the pen position, y is
    /// the top of the caret's line. Returns `(0.0, 0.0)` when there's no cursor.
    ///
    /// Every grapheme boundary maps to exactly one place:
    /// - at a wrapped line break, the caret is at the start of the next line;
    /// - in whitespace skipped at a wrap, it's at the end of the line above;
    /// - after a trailing newline, it's on the empty line below the text.
    pub fn cursor_position(&self) -> (f32, f32) {
        let Some(cursor) = self.cursor else { return (0.0, 0.0) };
        let (_, x, top) = self.locate(&self.lines(), cursor);
        (x, top)
    }

    /// Moves the cursor to the nearest position on the line above or below.
    /// Up on the first line goes to the start, down on the last to the end.
    pub fn move_cursor_vertically(&mut self, down: bool) {
        let Some(cursor) = self.cursor else { return };
        let lines = self.lines();
        let (line, x, _) = self.locate(&lines, cursor);

        let target = match (down, line.checked_sub(1)) {
            (true, _) => line + 1,
            (false, Some(above)) => above,
            (false, None) => { self.cursor = Some(0); return; }
        };

        // Past the last line: the end of the text (which is the empty line
        // after a trailing newline, when there is one).
        let Some(target) = lines.get(target) else {
            self.cursor = Some(self.text_len());
            return;
        };

        let nearest = target.3.iter()
            .min_by(|a, b| (a.1 - x).abs().total_cmp(&(b.1 - x).abs()))
            .map(|&(byte, _)| byte);

        if let Some(byte) = nearest { self.cursor = Some(byte); }
    }

    /// The caret's line index, x and line top for `cursor`. The index
    /// `lines.len()` is the empty line after a trailing newline.
    fn locate(&self, lines: &[Line], cursor: usize) -> (usize, f32, f32) {
        let cursor = cursor.min(self.text_len());

        if cursor == self.text_len() && self.ends_with_newline() {
            return (lines.len(), self.empty_line_x(), lines.iter().map(|l| l.1).sum());
        }

        // Stops increase across lines, so the caret is at the last stop
        // at or before the cursor.
        let mut top = 0.0;
        let mut found = None;
        for (i, line) in lines.iter().enumerate() {
            for &(byte, x) in &line.3 {
                if byte > cursor { return found.unwrap_or((i, x, top)); }
                found = Some((i, x, top));
            }
            top += line.1;
        }

        found.unwrap_or((0, self.empty_line_x(), 0.0))
    }

    /// Moves the cursor to the grapheme boundary nearest to `(x, y)`, in the
    /// same coordinates as `cursor_position`.
    pub fn cursor_click(&mut self, x: f32, y: f32) {
        let lines = self.lines();
        let height: f32 = lines.iter().map(|l| l.1).sum();

        if lines.is_empty() || (y >= height && self.ends_with_newline()) {
            self.cursor = Some(self.text_len());
            return;
        }

        // The line under `y`, clamped to the first and last lines.
        let mut top = 0.0;
        let mut target = &lines[lines.len() - 1];
        for line in &lines {
            if y < top + line.1 { target = line; break; }
            top += line.1;
        }

        let nearest = target.3.iter()
            .min_by(|a, b| (a.1 - x).abs().total_cmp(&(b.1 - x).abs()))
            .map(|&(byte, _)| byte);

        if let Some(byte) = nearest { self.cursor = Some(byte); }
    }

    /// Length in bytes of all spans joined; the largest valid cursor.
    pub fn text_len(&self) -> usize {
        self.spans.iter().map(|s| s.text.len()).sum()
    }

    fn ends_with_newline(&self) -> bool {
        self.spans.iter().rev().find(|s| !s.text.is_empty())
            .is_some_and(|s| s.text.ends_with('\n'))
    }

    /// The x where a caret sits on an empty line, given the alignment.
    fn empty_line_x(&self) -> f32 {
        match self.align {
            Align::Left => 0.0,
            Align::Center => self.width.map_or(0.0, |w| w / 2.0),
            Align::Right => self.width.unwrap_or(0.0),
        }
    }

    pub fn len(&self) -> usize {
        self.lines().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub(crate) fn lines(&self) -> Vec<Line> {
        // The cursor doesn't affect layout, so it's left out of the cache key;
        // otherwise every cursor move would lay the text out again.
        let mut hasher = DefaultHasher::new();
        self.spans.hash(&mut hasher);
        self.width.map(f32::to_bits).hash(&mut hasher);
        self.align.hash(&mut hasher);
        self.max_lines.hash(&mut hasher);

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

        // Byte offsets into all spans joined: where the current line starts
        // (used for the caret on an empty line) and where its last laid-out
        // grapheme ends (used for the caret at the end of the line).
        let mut line_start = 0usize;
        let mut line_end = 0usize;

        // Ends the current line, giving it a caret stop at its end (or one
        // stop at its start if it's empty), and starts the next line at `next`.
        let push = |lines: &mut Vec<Line>, line: &mut Line, lh: f32, line_start: &mut usize, line_end: usize, next: usize| {
            if line.2.is_empty() {
                line.1 = line.1.max(lh);
                line.3 = vec![(*line_start, 0.0)];
            } else {
                line.3.push((line_end, line.0));
            }

            line.2.iter_mut().for_each(|ch| ch.1 .1 += line.1);
            lines.push(line.take());
            *line_start = next;
        };

        let mut span_start = 0usize;
        let mut soft_wrapped = false;
        for s in &self.spans {
            let lm = s.font.horizontal_line_metrics(s.font_size).unwrap();
            let lh = s.line_height.unwrap_or(lm.new_line_size);

            let mut raw_start = span_start;
            for (i, raw) in s.text.split('\n').enumerate() {
                if i > 0 {
                    push(&mut lines, &mut line, lh, &mut line_start, line_end, raw_start);
                    soft_wrapped = false;
                }

                let mut word_start = raw_start;
                for word in raw.split_inclusive(char::is_whitespace) {
                    let mut ww = 0.0;

                    let glyphs: Vec<_> = UnicodeSegmentation::grapheme_indices(word, true)
                        .map(|(gi, g)| {
                            let is_emoji = is_emoji_grapheme(g);

                            let font = if is_emoji {
                                EMOJI_FONT.clone()
                            } else {
                                s.font.clone()
                            };

                            let (xmin, ymin, w, h, aw) = if is_emoji {
                                // fontdue can't read outline bounds out of a
                                // color emoji font, so measure via swash
                                // instead -- it renders the actual bitmap
                                // and reports its placement.
                                font.swash_metrics(g, s.font_size)
                                    .map(|m| (m.xmin, m.ymin, m.width, m.height, m.advance_width))
                                    .unwrap_or((0.0, 0.0, 0.0, 0.0, 0.0))
                            } else {
                                let mut aw = 0.0_f32;
                                let mut xmin = 0.0_f32;
                                let mut ymin = 0.0_f32;
                                let mut xmax = 0.0_f32;
                                let mut ymax = 0.0_f32;
                                let mut first = true;

                                for c in g.chars() {
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

                                (xmin, ymin, xmax - xmin, ymax - ymin, aw)
                            };

                            ww += aw + s.kerning;

                            (
                                word_start + gi,
                                g.len(),
                                g.to_string(),
                                font,
                                (xmin, ymin, w, h),
                                aw,
                                g.chars().all(char::is_whitespace),
                                is_emoji,
                            )
                        })
                        .collect();

                    if let Some(wmax) = self.width
                        && ww < wmax
                        && line.0 + ww > wmax
                        && !line.2.is_empty()
                    {
                        soft_wrapped = true;
                        push(&mut lines, &mut line, lh, &mut line_start, line_end, word_start);
                    }

                    for (byte, len, g, font, (xmin, ymin, w, h), aw, is_whitespace, is_emoji) in glyphs {
                        if let Some(wmax) = self.width {
                            if line.0 + aw + s.kerning > wmax && !line.2.is_empty() {
                                push(&mut lines, &mut line, lh, &mut line_start, line_end, byte);
                                soft_wrapped = true;
                            }

                            // Whitespace at the start of a wrapped line isn't drawn,
                            // so the line effectively starts after it.
                            if soft_wrapped && line.2.is_empty() && is_whitespace {
                                line_start = byte + len;
                                continue;
                            }
                        }

                        let y = lines.iter().fold(0.0, |h, l| h + l.1) - ymin - h + lm.descent;

                        // Regular glyphs are cached as alpha-only masks and
                        // rely on this color to tint them. Emoji are cached
                        // as already-colored bitmaps (from swash/CBDT), so
                        // tinting them would overwrite their real color --
                        // pass None to signal "use the texture as-is".
                        let color = if is_emoji { None } else { Some(s.color) };

                        // A caret stop before this grapheme, at the pen position.
                        line.3.push((byte, line.0));
                        line_end = byte + len;

                        line.2.push(Character(
                            g.clone(),
                            (line.0 + xmin, y, w, h),
                            font,
                            color,
                            lh,
                            aw,
                        ));

                        line.0 += aw + s.kerning;
                        line.1 = line.1.max(lh);
                    }

                    word_start += word.len();
                }

                raw_start += raw.len() + 1;
            }

            span_start += s.text.len();
        }

        if !line.2.is_empty() {
            push(&mut lines, &mut line, 0.0, &mut line_start, line_end, line_end);
        }

        // Where a line wraps with nothing skipped, its end and the next line's
        // start are the same byte. Keep only the next line's stop, so each
        // boundary has exactly one caret position.
        for i in 1..lines.len() {
            let next_start = lines[i].3.first().map(|c| c.0);
            let prev = &mut lines[i - 1].3;
            if prev.len() > 1 && prev.last().map(|c| c.0) == next_start {
                prev.pop();
            }
        }

        lines.iter_mut().for_each(|line| {
            let offset = match self.align {
                Align::Left => 0.0,
                Align::Center => self.width.map_or(0.0, |w| (w - line.0) / 2.0),
                Align::Right => self.width.map_or(0.0, |w| w - line.0),
            };

            line.2.iter_mut().for_each(|ch| ch.1 .0 += offset);
            line.3.iter_mut().for_each(|stop| stop.1 += offset);
            line.0 += offset;
        });

        if let Some(max) = self.max_lines {
            let len = lines.len();
            lines.truncate(max as usize);

            if len > max as usize {
                let y = lines.iter().fold(0.0, |h, l| h + l.1);

                if let Some(last) = lines.last_mut() {
                    last.2.truncate(last.2.len().saturating_sub(3));
                    // Keep the stops before each remaining grapheme plus one at
                    // the ellipsis; hidden text has no caret positions.
                    last.3.truncate(last.2.len() + 1);

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
    pub Font,
    pub Option<Color>,
    pub f32,
    pub f32,
);

#[derive(Debug, Clone, Default)]
pub(crate) struct Line(pub f32, pub f32, pub Vec<Character>, pub Vec<(usize, f32)>);

impl Line {
    fn take(&mut self) -> Self {
        let l = Line(self.0, self.1, self.2.drain(..).collect(), std::mem::take(&mut self.3));
        self.0 = 0.0;
        self.1 = 0.0;
        l
    }
}