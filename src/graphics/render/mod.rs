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

pub trait Render {
    fn new_texture(&mut self, width: i32, height: i32, data: Box<[u8]>) -> TextureHandle;
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
}