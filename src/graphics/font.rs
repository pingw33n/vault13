use bstring::bstr;
use enum_map_derive::Enum;
use std::collections::HashMap;
use std::ops::Range;

use crate::graphics::color::Rgb15;
use crate::graphics::render::{Canvas, Outline, TextureHandle};

#[derive(Clone, Copy, Debug, Enum, Eq, PartialEq)]
pub enum HorzAlign {
    Left,
    Center,
    Right,
}

impl Default for HorzAlign {
    fn default() -> Self {
        HorzAlign::Left
    }
}

#[derive(Clone, Copy, Debug, Enum, Eq, PartialEq)]
pub enum VertAlign {
    Top,
    Middle,
    Bottom,
}

impl Default for VertAlign {
    fn default() -> Self {
        VertAlign::Top
    }
}

#[derive(Clone, Copy, Debug, Enum, Eq, PartialEq)]
pub enum OverflowMode {
    Truncate,
    WordWrap,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Overflow {
    pub size: i32,
    pub mode: OverflowMode,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DrawOptions {
    pub horz_align: HorzAlign,
    pub vert_align: VertAlign,
    pub dst_color: Option<Rgb15>,
    pub outline: Option<Outline>,
    pub horz_overflow: Option<Overflow>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FontKey {
    pub id: u32,
    pub antialiased: bool,
}

impl FontKey {
    pub fn non_antialiased(id: u32) -> Self {
        Self {
            id,
            antialiased: false,
        }
    }

    pub fn antialiased(id: u32) -> Self {
        Self {
            id,
            antialiased: true,
        }
    }
}

pub struct Font {
    pub height: i32,
    pub horz_spacing: i32,
    pub vert_spacing: i32,
    pub glyphs: Box<[Glyph]>,
}

impl Font {
    pub fn measure_max_line_width(&self, text: &bstr) -> i32 {
        let mut max_line_width = 0;
        for line in text.lines() {
            let mut line_width = 0;
            for &c in line {
                line_width += self.glyphs.get(c as usize)
                    .map(|g| g.width + self.horz_spacing)
                    .unwrap_or(0);
            }
            if line_width > max_line_width {
                max_line_width = line_width;
            }
        }
        max_line_width
    }

    pub fn measure_text_height(&self, text: &bstr) -> i32 {
        text.lines().count() as i32 * self.height
    }

    pub fn vert_advance(&self) -> i32 {
        self.height + self.vert_spacing
    }

    pub fn line_ranges<'a>(&'a self, text: &'a bstr, horz_overflow: Option<Overflow>) -> LineRanges
    {
        LineRanges(LineRanges0::new(self, text, horz_overflow))
    }

    pub fn lines<'a, 'b>(&'a self, text: &'b bstr, horz_overflow: Option<Overflow>) -> Lines<'a, 'b>
    {
        Lines(LineRanges0::new(self, text, horz_overflow))
    }

    pub fn draw(&self, canvas: &mut Canvas, text: &bstr, x: i32, y: i32, color: Rgb15,
            options: &DrawOptions) {
        let mut y = match options.vert_align {
            VertAlign::Top => y,
            VertAlign::Middle => y - self.measure_text_height(text) / 2,
            VertAlign::Bottom => y - self.measure_text_height(text),
        };

        for line in self.lines(text, options.horz_overflow) {
            let mut x = match options.horz_align {
                HorzAlign::Left => x,
                HorzAlign::Center => x - self.measure_max_line_width(line) / 2,
                HorzAlign::Right => x - self.measure_max_line_width(line),
            };
            for &c in line {
                let glyph = &self.glyphs[c as usize];
                let y = y + self.height - glyph.height;

                canvas.draw_masked_color(color, options.dst_color, x, y, &glyph.texture);

                if let Some(outline) = options.outline {
                    canvas.draw_outline(&glyph.texture, x, y, outline);
                }
                x += glyph.width + self.horz_spacing;
            }
            y += self.vert_advance();
        }
    }
}

struct LineRanges0<'a, 'b> {
    font: &'a Font,
    text: &'b bstr,
    horz_overflow: Option<Overflow>,
    i: usize,
}

impl<'a, 'b> LineRanges0<'a, 'b> {
    pub fn new(font: &'a Font, text: &'b bstr, horz_overflow: Option<Overflow>) -> Self {
        Self {
            font,
            text,
            horz_overflow,
            i: 0,
        }
    }
}

impl Iterator for LineRanges0<'_, '_> {
    type Item = Range<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut cur_width = 0;
        let start = self.i;
        let mut end;
        loop {
            end = self.i;

            if self.i == self.text.len() {
                break;
            }

            let c = self.text[self.i];

            self.i += 1;

            if c == b'\r' {
                if self.i < self.text.len() && self.text[self.i] == b'\n' {
                    self.i += 1;
                }
                break;
            }
            if c == b'\n' {
                break;
            }

            let glyph = &self.font.glyphs[c as usize];

            cur_width += glyph.width + self.font.horz_spacing;

            if let Some(Overflow { size, mode }) = self.horz_overflow {
                if cur_width > size {
                    match mode {
                        OverflowMode::Truncate => {}
                        OverflowMode::WordWrap => {
                            if let Some(i) = self.text[start..end].iter()
                                .rposition(|&c| c == b' ')
                                .map(|i| start + i)
                            {
                                end = i;
                                self.i = i + 1;
                            } else {
                                self.i -= 1;
                            }
                        }
                    }
                    break;
                }
            }
        }
        if start < self.text.len() {
            let mut start = start;
            // Trim leading whitespace if this is not the first line.
            while start > 0 && start < end && self.text[start] == b' ' {
                start += 1;
            }
            // Trim trailing whitespace.
            while end > start + 1 && self.text[end - 1] == b' ' {
                end -= 1;
            }
            Some(Range { start, end })
        } else {
            None
        }
    }
}

pub struct LineRanges<'a>(LineRanges0<'a, 'a>);

impl Iterator for LineRanges<'_> {
    type Item = Range<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

pub struct Lines<'a, 'b>(LineRanges0<'a, 'b>);

impl<'a, 'b> Iterator for Lines<'a, 'b> {
    type Item = &'b bstr;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|r| &self.0.text[r])
    }
}

pub struct Glyph {
    pub width: i32,
    pub height: i32,
    pub texture: TextureHandle,
}

pub struct Fonts {
    fonts: HashMap<FontKey, Font>,
}

impl Fonts {
    pub fn new() -> Self {
        Self {
            fonts: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: FontKey, font: Font) {
        let existing = self.fonts.insert(key, font);
        assert!(existing.is_none());
    }

    pub fn get(&self, key: FontKey) -> &Font {
        &self.fonts[&key]
    }
}