use bstring::bstr;
use std::collections::HashMap;

use graphics::color::Rgb15;
use graphics::render::{Outline, Renderer, TextureHandle};

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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DrawOptions {
    pub horz_align: HorzAlign,
    pub vert_align: VertAlign,
    pub dst_color: Option<Rgb15>,
    pub outline: Option<Outline>,
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

    pub fn draw(&self, renderer: &mut Renderer, text: &bstr, x: i32, y: i32, color: Rgb15,
            options: &DrawOptions) {
        let mut y = match options.vert_align {
            VertAlign::Top => y,
            VertAlign::Middle => y - self.measure_text_height(text) / 2,
            VertAlign::Bottom => y - self.measure_text_height(text),
        };

        for line in text.lines() {
            let mut x = match options.horz_align {
                HorzAlign::Left => x,
                HorzAlign::Center => x - self.measure_max_line_width(line) / 2,
                HorzAlign::Right => x - self.measure_max_line_width(line),
            };
            for &c in line {
                let glyph = &self.glyphs[c as usize];
                let y = y + self.height - glyph.height;

                renderer.draw_masked_color(color, options.dst_color, x, y, &glyph.texture);

                if let Some(outline) = options.outline {
                    renderer.draw_outline(&glyph.texture, x, y, outline);
                }
                x += glyph.width + self.horz_spacing;
            }
            y += self.height + self.vert_spacing;
        }
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