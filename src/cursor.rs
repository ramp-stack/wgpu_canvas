use ramp_glyphon::ramp_text::Cursor as CosmicCursor;
use ramp_glyphon::{Affinity, Buffer};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CursorAction {
    GetPosition,
    GetIndex,
    MoveLeft,
    MoveRight,
    MoveNewline,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cursor {
    pub line: usize, // Layout line number
    pub index: usize, // Glyph index
    pub affinity: Affinity, // Appear before or after glyph
    pub position: Option<(f32, f32)>,
    // pub size: (f32, f32), // width and height
    // pub color: Color, // Color of cursor
}

impl Default for Cursor {
    fn default() -> Self {
        Cursor { line: 0, index: 0, affinity: Affinity::Before, position: None }
    }
}

impl Cursor {
    pub fn from(cursor: CosmicCursor) -> Self {
        Cursor { line: cursor.line, index: cursor.index, affinity: cursor.affinity, position: None }
    }

    pub fn get_index(&mut self, buffer: &Buffer) -> usize {
        buffer.layout_runs().enumerate().flat_map(|(line_idx, run)| {
            match line_idx < self.line {
                true => run.glyphs.to_vec(),
                false if line_idx == self.line => {
                    run.glyphs.iter().take_while(|glyph| glyph.end <= self.index).cloned().collect()
                },
                false => Vec::new()
            }
        }).count() + self.line
    }

    pub fn position(&mut self, buffer: &Buffer) {
        let mut x_pos = 0.0;
        let mut line_h = 0.0;
        for (i, run) in buffer.layout_runs().enumerate() {
            line_h = run.line_height;
            if i == self.line {
                for glyph in run.glyphs {
                    if glyph.start <= self.index && self.index < glyph.end {
                        self.position = Some((x_pos, line_h*(self.line+1) as f32));
                        return;
                    }
                    x_pos += glyph.w;
                }
                self.position = Some((x_pos, line_h*(self.line+1) as f32));
                return;
            } 
        }
        // Fallback, was a newline with no glyph
        self.position = Some((x_pos, line_h*(self.line+1) as f32));
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
        self.position(buffer);
    }

    pub fn move_left(&mut self, buffer: &Buffer) {
        let total_lines = buffer.layout_runs().collect::<Vec<_>>().len();
        if self.line >= total_lines {
            self.line = total_lines.saturating_sub(1);
            self.index = buffer.layout_runs().nth(self.line).map_or(0, |r| r.glyphs.len());
            return;
        }

        for (i, _run) in buffer.layout_runs().enumerate() {
            if i == self.line {
                if self.index < 1 {
                    if self.line > 0 {
                        let prev_run = buffer.layout_runs().nth(self.line - 1).unwrap();
                        self.line -= 1;
                        if !prev_run.glyphs.is_empty() {
                            self.index = prev_run.glyphs.len();
                        }
                    }
                } else {
                    self.index -= 1;
                }
                break;
            }
        }
        
        self.position(buffer);
    }

    pub fn move_newline(&mut self, buffer: &Buffer) {
        self.line += 1;
        self.index = 0;
        self.position(buffer);
    }

    pub fn new_from_click(buffer: &Buffer, x: f32, y: f32) -> Option<Cursor> {
        use unicode_segmentation::UnicodeSegmentation;
        let mut new_cursor_opt = None;
        let mut runs = buffer.layout_runs().peekable();
        let mut first_run = true;
        for (i, run) in buffer.layout_runs().enumerate() {
            let line_top = run.line_top;
            let line_height = run.line_height;

            if first_run && y < line_top {
                first_run = false;
                let new_cursor = Cursor::from(CosmicCursor::new(run.line_i, 0));
                new_cursor_opt = Some(new_cursor);
            } else if y >= line_top && y < line_top + line_height {
                let mut new_cursor_glyph = run.glyphs.len();
                let mut new_cursor_char = 0;
                let mut new_cursor_affinity = Affinity::After;

                let mut first_glyph = true;

                'hit: for (index, glyph) in run.glyphs.iter().enumerate() {
                    if first_glyph {
                        first_glyph = false;
                        if (run.rtl && x > glyph.x) || (!run.rtl && x < 0.0) {
                            new_cursor_glyph = 0;
                            new_cursor_char = 0;
                        }
                    }
                    if x >= glyph.x && x <= glyph.x + glyph.w {
                        new_cursor_glyph = index;

                        let cluster = &run.text[glyph.start..glyph.end];
                        let total = cluster.grapheme_indices(true).count();
                        let mut egc_x = glyph.x;
                        let egc_w = glyph.w / (total as f32);
                        for (egc_i, egc) in cluster.grapheme_indices(true) {
                            if x >= egc_x && x <= egc_x + egc_w {
                                new_cursor_char = egc_i;

                                let right_half = x >= egc_x + egc_w / 2.0;
                                if right_half != glyph.level.is_rtl() {
                                    new_cursor_char += egc.len();
                                    new_cursor_affinity = Affinity::Before;
                                }
                                break 'hit;
                            }
                            egc_x += egc_w;
                        }

                        let right_half = x >= glyph.x + glyph.w / 2.0;
                        if right_half != glyph.level.is_rtl() {
                            new_cursor_char = cluster.len();
                            new_cursor_affinity = Affinity::Before;
                        }
                        break 'hit;
                    }
                }

                let mut new_cursor = Cursor::from(CosmicCursor::new(i, 0));

                match run.glyphs.get(new_cursor_glyph) {
                    Some(glyph) => {
                        new_cursor.index = glyph.start + new_cursor_char;
                        new_cursor.affinity = new_cursor_affinity;
                    }
                    None => {
                        if let Some(glyph) = run.glyphs.last() {
                            new_cursor.index = glyph.end;
                            new_cursor.affinity = Affinity::Before;
                        }
                    }
                }

                new_cursor_opt = Some(new_cursor);

                break;
            } else if runs.peek().is_none() && y > run.line_y {
                let mut new_cursor = Cursor::from(CosmicCursor::new(run.line_i, 0));
                if let Some(glyph) = run.glyphs.last() {
                    new_cursor = if run.rtl {
                        Cursor::from(CosmicCursor::new_with_affinity(run.line_i, glyph.end, Affinity::Before))
                    } else {
                        Cursor::from(CosmicCursor::new_with_affinity(run.line_i, glyph.start, Affinity::After))
                    }
                }
                new_cursor_opt = Some(new_cursor);
            }
        };

        new_cursor_opt
    }
}
