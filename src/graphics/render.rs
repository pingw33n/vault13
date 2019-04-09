pub mod software;

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;
use std::time::Instant;

use crate::graphics::color::Rgb15;
use crate::graphics::font::{self, FontKey};
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

    fn draw(&mut self, tex: &TextureHandle, x: i32, y: i32, light: u32);
    fn draw_multi_light(&mut self, tex: &TextureHandle, x: i32, y: i32, lights: &[u32]);

    /// Draws the specified `texture` masked using the specified `mask`.
    /// `mask` values are in range [0..128]. 0 is fully opaque, 128 is fully transparent.
    fn draw_masked(&mut self, texture: &TextureHandle, x: i32, y: i32,
                   mask: &TextureHandle, mask_x: i32, mask_y: i32,
                   light: u32);

    /// Alpha blends from `src` color to `dst` color with alpha mask specified by the `mask.
    /// If `dst` is `None` the current color of pixels in back buffer is used.
    /// `color`. `mask` values are in range [0..7]. Note the meaning here is inverted compared to
    /// `draw_masked()`: 0 is fully transparent `src` (and fully opaque `dst`),
    /// 7 is fully opaque `src` (and fully transparent `dst`).
    fn draw_masked_color(&mut self, src: Rgb15, dst: Option<Rgb15>, x: i32, y: i32,
                         mask: &TextureHandle);

    fn draw_translucent(&mut self, tex: &TextureHandle, x: i32, y: i32, color: Rgb15, light: u32);
    fn draw_translucent_dark(&mut self, tex: &TextureHandle, x: i32, y: i32, color: Rgb15, light: u32);
    fn draw_outline(&mut self, tex: &TextureHandle, x: i32, y: i32, outline: Outline);
    fn draw_text(&mut self, text: &[u8], x: i32, y: i32, font: FontKey, color: Rgb15,
        options: &font::DrawOptions);
}