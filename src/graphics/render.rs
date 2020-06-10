pub mod software;

use bstring::bstr;
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;
use std::time::Instant;

use crate::graphics::{Point, Rect};
use crate::graphics::color::Rgb15;
use crate::graphics::font::{self, FontKey, Fonts};
use crate::util::SmKey;

#[derive(Clone)]
pub struct TextureHandle(Rc<TextureHandleInner>);

impl fmt::Debug for TextureHandle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TextureHandle@{:?}", self.0.key)
    }
}

#[derive(Clone)]
struct TextureHandleInner {
    key: SmKey,
    drop_list: Rc<RefCell<Vec<SmKey>>>,
}

impl Drop for TextureHandleInner {
    fn drop(&mut self) {
        self.drop_list.borrow_mut().push(self.key);
    }
}

#[derive(Clone)]
pub struct TextureFactory(TextureFactoryInner);

#[derive(Clone)]
enum TextureFactoryInner {
    Software(software::Textures),
}

impl TextureFactory {
    pub fn new_texture(&self, width: i32, height: i32, data: Box<[u8]>) -> TextureHandle {
        match self.0 {
            TextureFactoryInner::Software(ref i) => i.new_texture(width, height, data),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outline {
    /// If `trans_color` is not `None`, outline will have translucency effect of that color.
    Fixed { color: Rgb15, trans_color: Option<Rgb15> },

    /// Cycles colors vertically in [start..start + len) range of color indices.
    /// The whole range is mapped to the whole texture height.
    Cycled { start: u8, len: u8 },
}

pub trait Canvas {
    fn cleanup(&mut self);
    fn present(&mut self);
    fn update(&mut self, time: Instant);

    fn fonts(&self) -> &Rc<Fonts>;

    fn set_clip_rect(&mut self, rect: Rect);
    fn reset_clip_rect(&mut self);

    fn clear(&mut self, color: Rgb15);

    fn draw(&mut self, tex: &TextureHandle, pos: Point, light: u32);
    fn draw_multi_light(&mut self, tex: &TextureHandle, pos: Point, lights: &[u32]);

    /// Draws the specified `texture` masked using the specified `mask`.
    /// `mask` values are in range [0..128]. 0 is fully opaque, 128 is fully transparent.
    fn draw_masked(&mut self, texture: &TextureHandle, pos: Point,
                   mask: &TextureHandle, mask_pos: Point,
                   light: u32);

    /// Alpha blends from `src` color to `dst` color with alpha mask specified by the `mask.
    /// If `dst` is `None` the current color of pixels in back buffer is used.
    /// `color`. `mask` values are in range [0..7]. Note the meaning here is inverted compared to
    /// `draw_masked()`: 0 is fully transparent `src` (and fully opaque `dst`),
    /// 7 is fully opaque `src` (and fully transparent `dst`).
    fn draw_masked_color(&mut self, src: Rgb15, dst: Option<Rgb15>, pos: Point,
                         mask: &TextureHandle);

    /// Similar to `draw_masked_color()` but the `mask` specifies combined alpha and lightening
    /// values. This is used for drawing screen glare effect in dialog window.
    fn draw_highlight(&mut self, color: Rgb15, pos: Point, mask: &TextureHandle);

    fn draw_translucent(&mut self, tex: &TextureHandle, pos: Point, color: Rgb15, light: u32);
    fn draw_translucent_dark(&mut self, tex: &TextureHandle, pos: Point, color: Rgb15, light: u32);
    fn draw_outline(&mut self, tex: &TextureHandle, pos: Point, outline: Outline);
    fn draw_text(&mut self, text: &bstr, pos: Point, font: FontKey, color: Rgb15,
        options: &font::DrawOptions);
}