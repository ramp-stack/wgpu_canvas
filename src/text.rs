use fontdue::{FontSettings, Metrics, LineMetrics};

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hasher, Hash};
use std::sync::Arc;
use std::cell::RefCell;
use std::rc::Rc;

use super::{Area, Color, Image, Shape, ImageAtlas, Atlas};

type SpanInfo = (u64, f32, Option<f32>, Font);

pub type Cursor = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Align {Left, Center, Right}

#[derive(Debug, Clone, PartialEq)]
pub struct Span{
    pub text: String, 
    pub font_size: f32,
    pub line_height: Option<f32>,
    pub font: Font,
    pub color: Color
}

impl Span {
    pub fn new(text: String, font_size: f32, line_height: Option<f32>, font: Font, color: Color) -> Self {
        Span{text, font_size, line_height, font, color}
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Character(char, (f32, f32, f32, f32), Font, Color, f32, f32);

#[derive(Debug, Clone, PartialEq, Default)]
struct Line(f32, f32, Vec<Character>);
impl Line {
    fn take(&mut self) -> Self {
        let l = Line(self.0, self.1, self.2.drain(..).collect());
        self.0 = 0.0;
        self.1 = 0.0;
        l
    }
}

type Lines = (Vec<Line>, (Vec<SpanInfo>, Option<f32>), u64);

#[derive(Debug, Clone, PartialEq)]
pub struct Text{
    pub spans: Vec<Span>,
    pub width: Option<f32>,
    pub align: Align,
    pub cursor: Option<Cursor>,
    pub scale: f32,
    lines: Rc<RefCell<Lines>>,
}


impl Text {
    fn hash_size(&self) -> (Vec<SpanInfo>, Option<f32>) {
        (self.spans.iter().map(|s| {
            let mut state = DefaultHasher::new();
            s.text.hash(&mut state);
            (state.finish(), s.font_size, s.line_height, s.font.clone())
        }).collect::<Vec<_>>(),
        self.width)
    }
    fn hash_rest(&self) -> u64 {
        let mut state = DefaultHasher::new();
        self.spans.iter().for_each(|s| s.color.hash(&mut state));
        self.align.hash(&mut state);
        state.finish()
    }

    pub fn new(spans: Vec<Span>, width: Option<f32>, align: Align) -> Self {
        Text{spans, width, align, cursor: None, scale: 1.0, lines: Rc::new(RefCell::new((Vec::new(), (Vec::new(), None), 0)))}
    }

    pub fn size(&self, mut ctx: impl AsMut<Atlas>) -> (f32, f32) {
        let size_state = self.hash_size();
        if size_state != self.lines.borrow().1 {
            let lines = self.lines(&mut ctx.as_mut().text);
            let rest_state = self.hash_rest();
            *self.lines.borrow_mut() = (lines, size_state, rest_state);
        }
        self.lines.borrow().0.iter().fold((0.0, 0.0), |(w, h), line| (w.max(line.0), h+line.1))
    }

    pub fn cursor_position(&self) -> (f32, f32) {
        let ls = &self.lines.borrow().0;
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

        for line in self.lines.borrow().0.iter() {
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



    fn lines(&self, atlas: &mut TextAtlas) -> Vec<Line> {
        let mut lines = Vec::new();
        let mut current_line = Line::default();
        self.spans.iter().for_each(|s| {
            let lm = atlas.line_metrics(&s.font);
            let lh = s.line_height.unwrap_or_else(|| lm.new_line_size * s.font_size);
            s.text.split('\n').into_iter().for_each(|raw_line| {
                raw_line.split_inclusive(|c: char| c.is_whitespace()).into_iter().for_each(|word| {
                    let mut word_width = 0.0;

                    let glyphs: Vec<_> = word.chars().map(|c| {
                        let m = atlas.metrics(&s.font, c, s.font_size);
                        let aw = m.advance_width;
                        word_width += aw;
                        (c, (m.bounds.xmin, m.bounds.ymin, m.bounds.width, m.bounds.height), aw)
                    }).collect();

                    if let Some(width) = self.width {
                        if current_line.0 + word_width > width && !current_line.2.is_empty() {
                            current_line.2.iter_mut().for_each(|ch| ch.1.1 += current_line.1);
                            lines.push(current_line.take());
                        }
                    }

                    glyphs.into_iter().for_each(|(c, (xmin, ymin, width, height), aw)| {
                        current_line.2.push(Character(c,
                            (current_line.0 + xmin,(lines.iter().fold(0.0, |h, l| h + l.1) - ymin - height) + (lm.descent * s.font_size), width, height),
                            s.font.clone(), s.color, lh, aw
                        ));
                        current_line.0 += aw;
                        current_line.1 = current_line.1.max(lh);
                    });
                });

                if current_line.2.is_empty() { current_line.1 = current_line.1.max(lh); }
                current_line.2.iter_mut().for_each(|ch| ch.1.1 += current_line.1);
                lines.push(current_line.take());

            })
        });

        if !current_line.2.is_empty() {
            current_line.2.iter_mut().for_each(|ch| ch.1.1 += current_line.1);
            lines.push(current_line.take());
        }

        lines.iter_mut().for_each(|line| {
            let offset_x = match self.align {
                Align::Left => 0.0,
                Align::Center => self.width.map_or(0.0, |w| (w - line.0) / 2.0),
                Align::Right => self.width.map_or(0.0, |w| w - line.0),
            };
            line.2.iter_mut().for_each(|ch| ch.1.0 += offset_x);
            line.0 += offset_x;
        });
        
        lines
    }
}

// self.spans.iter().for_each(|s| s.text.chars().for_each(|c| {
//     if c == '\n' {
//         current_line.2.iter_mut().for_each(|ch| ch.1.1 += current_line.1);
//         lines.push(current_line.take());
//     } else {
//         let lm = atlas.line_metrics(&s.font);
//         let lh = s.line_height.unwrap_or_else(|| lm.new_line_size * s.font_size);
//         let m = atlas.metrics(&s.font, c, s.font_size);
//         let aw = m.advance_width;
//         let m = m.bounds;
//         if let Some(width) = self.width {
//             if current_line.0+aw > width {
//                 current_line.2.iter_mut().for_each(|ch| ch.1.1 += current_line.1);
//                 lines.push(current_line.take());
//             }
//         }
//         current_line.2.push(Character(
//             c, (current_line.0 + m.xmin, ((lines.iter().fold(0.0, |h, l| h+l.1) - m.ymin) - m.height) + (lm.descent * s.font_size), m.width, m.height), s.font.clone(), s.color, s.font_size
//         ));
//         current_line.0 += aw;
//         current_line.1 = current_line.1.max(lh);

//     }
// }));

pub type Font = Arc<u64>;
//type Atlas = TexturePacker<'static, RgbaImage, char>;

type MetricsMap = HashMap<(char, u32), Metrics>;
type ImageMap = HashMap<(char, u32), Option<Image>>;

//TODO: Add back atlas and combine all minor images into one large one
#[derive(Default)]
pub struct TextAtlas{
    fonts: HashMap<Font, (
        MetricsMap,
        ImageMap,
        LineMetrics,
        fontdue::Font
    )>,//(Atlas, ..
}

impl TextAtlas {
    pub fn add(&mut self, raw_font: &[u8]) -> Result<Font, &'static str> {
        let mut hasher = DefaultHasher::new();
        raw_font.hash(&mut hasher);
        let id = hasher.finish();
        match self.fonts.keys().find_map(|k| (**k == id).then(|| k.clone())) {
            Some(id) => Ok(id),
            None => {
                let id = Arc::new(id);
                let font = fontdue::Font::from_bytes(raw_font, FontSettings{scale: 160.0, ..Default::default()})?;
                self.fonts.insert(id.clone(), (
                //TexturePacker::new_skyline(TexturePackerConfig{
                //    texture_padding: 1, allow_rotation: false, trim: false, ..Default::default()
                //}),
                    HashMap::new(),
                    HashMap::new(),
                    font.horizontal_line_metrics(1.0).expect("Not a Horizontal Font"),//Scale
                    font
                ));
                Ok(id)
            }
        }
    }

    fn trim(&mut self) {
        self.fonts = self.fonts.drain().filter(|(k, _)| (Arc::strong_count(k) > 1)).collect();
    }

    fn line_metrics(&self, font: &Font) -> LineMetrics {
        self.fonts.get(font).expect("Font Unloaded").2
    }

    fn metrics(&mut self, font: &Font, c: char, scale: f32) -> Metrics {
        let (ms, _, _, f) = self.fonts.get_mut(font).expect("Font Unloaded");
        let k = (c, (scale*1_000_000.0) as u32);//f32 cannot be saved in hash map
        match ms.get(&k) {
            Some(m) => *m,
            None => {
                let m = f.metrics(c, scale);//Scale
                ms.insert(k, m);
                m
            }
        }
    }

    fn get_image(&mut self, image_atlas: &mut ImageAtlas, font: &Font, c: char, scale: f32) -> Option<Image> {//Include Image Offset and crop from atlas
        let (_, is, _, f) = self.fonts.get_mut(font).expect("Font Unloaded");
        let k = (c, (scale*1_000_000.0) as u32);//f32 cannot be saved in hash map
        match is.get(&k) {
            Some(i) => i.clone(),
            None => {
                //TODO: If scale factor changes need to re rasterize fonts
                //TODO: Choose 3 font sizes that should be used for all rasterization if needed
                //TODO: Or use one size that is the largest neccessary including max scale_factor 
                let (m, b) = f.rasterize(c, scale*2.0);
                let b: Vec<_> = b.iter().flat_map(|a| [0, 0, 0, *a]).collect();
                let image = b.iter().any(|a| *a != 0).then(|| {
                    let img = image::RgbaImage::from_raw(m.width as u32, m.height as u32, b).unwrap();
                    image_atlas.add(img)
                });
                //atlas.pack_own(*ch, img).unwrap();
                is.insert(k, image.clone());
                image
            }
        }
    }

    pub(crate) fn prepare_images(&mut self, image_atlas: &mut ImageAtlas, text_areas: Vec<(u16, Area, Text)>) -> Vec<(u16, Area, Shape, Image, Option<Color>)> {
        self.trim();
        text_areas.into_iter().flat_map(|(z, a, t)| {
            let size_state = t.hash_size();
            let rest_state = t.hash_rest();
            if size_state != t.lines.borrow().1 || rest_state != t.lines.borrow().2 {
                t.lines(self);
            }
            let scale = t.scale;
            t.lines.borrow().0.iter().flat_map(|line| line.2.iter().flat_map(|ch| {
                self.get_image(image_atlas, &ch.2, ch.0, ch.4*scale).map(|img| {
                    let a = Area((a.0.0+(ch.1.0*scale), a.0.1+(ch.1.1*scale)), a.1);
                    let shape = Shape::Rectangle(0.0, (ch.1.2*scale, ch.1.3*scale));
                    (z, a, shape, img, Some(ch.3))
                })
            }).collect::<Vec<_>>()).collect::<Vec<_>>()
      }).collect()
    } 
}
