use slotmap::DefaultKey;
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;
use std::time::Instant;

use graphics::color::Rgb15;

pub mod software;

#[derive(Clone)]
pub struct TextureHandle(Rc<TextureHandleInner>);

impl fmt::Debug for TextureHandle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TextureHandle@{:?}", self.0.key)
    }
}

#[derive(Clone)]
struct TextureHandleInner {
    key: DefaultKey,
    drop_list: Rc<RefCell<Vec<DefaultKey>>>,
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
    Color(Rgb15),

    /// Cycles colors vertically in [start..start + len) range of color indices.
    /// The whole range is mapped to the whole texture height.
    ColorCycle { start: u8, len: u8 },

    /// Outline with translucency effect of `trans_color`.
    Translucent { color: Rgb15, trans_color: Rgb15 },
}

pub trait Renderer {
    fn new_texture_factory(&self) -> TextureFactory;

    fn cleanup(&mut self);
    fn present(&mut self);
    fn update(&mut self, time: Instant);

    fn draw(&mut self, tex: &TextureHandle, x: i32, y: i32, light: u32);
    fn draw_multi_light(&mut self, tex: &TextureHandle, x: i32, y: i32, lights: &[u32]);
    fn draw_masked(&mut self, tex: &TextureHandle, x: i32, y: i32,
                   mask: &TextureHandle, mask_x: i32, mask_y: i32,
                   light: u32);
    fn draw_translucent(&mut self, tex: &TextureHandle, x: i32, y: i32, color: Rgb15, light: u32);
    fn draw_translucent_dark(&mut self, tex: &TextureHandle, x: i32, y: i32, color: Rgb15, light: u32);
    fn draw_outline(&mut self, tex: &TextureHandle, x: i32, y: i32, outline: Outline);
}