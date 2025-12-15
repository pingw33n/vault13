use bstring::bstr;
use enum_map_derive::Enum;
use std::collections::HashMap;
use std::ops::Range;

use crate::graphics::Point;
use crate::graphics::color::Rgb15;
use crate::graphics::render::{Canvas, Outline, TextureHandle};

#[derive(Clone, Copy, Debug, Default, Enum, Eq, PartialEq)]
pub enum HorzAlign {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, Default, Enum, Eq, PartialEq)]
pub enum VertAlign {
    #[default]
    Top,
    Middle,
    Bottom,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OverflowBoundary {
    Char,
    Word,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OverflowAction {
    Truncate,
    Wrap,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Overflow {
    pub size: i32,
    pub boundary: OverflowBoundary,
    pub action: OverflowAction,
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
    pub const fn non_antialiased(id: u32) -> Self {
        Self {
            id,
            antialiased: false,
        }
    }

    pub const fn antialiased(id: u32) -> Self {
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
    /// Return width of a line of text without applying wrapping.
    pub fn line_width(&self, line: &bstr) -> i32 {
        let mut r = 0;
        for &c in line {
            r += self.glyphs.get(c as usize)
                .map(|g| g.width + self.horz_spacing)
                .unwrap_or(0);
        }
        r
    }

    /// Returns width of the `text` with wrapping applied. The result is the width of the longest
    /// line in text.
    pub fn text_width(&self, text: &bstr, horz_overflow: Option<Overflow>) -> i32 {
        self.lines(text, horz_overflow)
            .map(|l| self.line_width(l))
            .max()
            .unwrap_or(0)
    }

    /// Returns height of the `text` with wrapping applied.
    pub fn text_height(&self, text: &bstr, horz_overflow: Option<Overflow>) -> i32 {
        self.lines(text, horz_overflow).count() as i32 * self.vert_advance()
    }

    pub fn vert_advance(&self) -> i32 {
        self.height + self.vert_spacing
    }

    pub fn line_ranges<'a>(&'a self, text: &'a bstr, horz_overflow: Option<Overflow>) -> LineRanges<'a>
    {
        LineRanges(LineRanges0::new(self, text, horz_overflow))
    }

    pub fn lines<'a, 'b>(&'a self, text: &'b bstr, horz_overflow: Option<Overflow>) -> Lines<'a, 'b>
    {
        Lines(LineRanges0::new(self, text, horz_overflow))
    }

    pub fn draw(&self, canvas: &mut dyn Canvas, text: &bstr, pos: Point, color: Rgb15,
            options: &DrawOptions) {
        let mut y = match options.vert_align {
            VertAlign::Top => pos.y,
            VertAlign::Middle => pos.y - self.text_height(text, options.horz_overflow) / 2,
            VertAlign::Bottom => pos.y - self.text_height(text, options.horz_overflow),
        };

        for line in self.lines(text, options.horz_overflow) {
            let mut x = match options.horz_align {
                HorzAlign::Left => pos.x,
                HorzAlign::Center => pos.x - self.text_width(line, options.horz_overflow) / 2,
                HorzAlign::Right => pos.x - self.text_width(line, options.horz_overflow),
            };
            for &c in line {
                let glyph = &self.glyphs[c as usize];
                let y = y + self.height - glyph.height;

                canvas.draw_masked_color(color, options.dst_color, Point::new(x, y), &glyph.texture);

                if let Some(outline) = options.outline {
                    canvas.draw_outline(&glyph.texture, Point::new(x, y), outline);
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

    fn can_wrap_after(c: u8) -> bool {
        c.is_ascii_whitespace() || c == b'-'
    }
}

impl Iterator for LineRanges0<'_, '_> {
    type Item = Range<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut cur_width = 0;
        let start = self.i;
        let mut overflown = false;
        let mut end = 0;
        loop {
            if !overflown {
                end = self.i;
            }

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

            if let Some(Overflow { size, boundary, action }) = self.horz_overflow
                && !overflown && cur_width > size 
            {
                overflown = true;
                match boundary {
                    OverflowBoundary::Char => {
                        end = end.saturating_sub(1);
                    }
                    OverflowBoundary::Word => {
                        if let Some(i) = self.text[start..end].iter()
                            .rposition(|&c| Self::can_wrap_after(c))
                            .map(|i| start + i)
                        {
                            end = i;
                            self.i = i + 1;
                        } else {
                            self.i -= 1;
                        }
                    }
                }
                match action {
                    OverflowAction::Truncate => {} // keep looking for line end
                    OverflowAction::Wrap => break,
                }
            }
        }
        if start < self.text.len() {
            let mut start = start;

            // Trim leading whitespace of a word-wrapped line.
            if matches!(self.horz_overflow,
                Some(Overflow { boundary: OverflowBoundary::Word, action: OverflowAction::Wrap, .. }))
            {
                while start > 0 && start < end && self.text[start].is_ascii_whitespace() {
                    start += 1;
                }
            }

            // Trim trailing whitespace.
            while end > start + 1 && self.text[end - 1].is_ascii_whitespace() {
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
