use sdl2::pixels::PixelFormatEnum;
use sdl2::render::{Texture as SdlTexture, WindowCanvas};
use slotmap::{SecondaryMap, SlotMap};
use std::cmp;
use std::rc::Rc;
use std::cell::{Ref, RefCell};

use super::*;
use crate::graphics::color::Color8;
use crate::graphics::color::palette::Palette;
use crate::graphics::color::palette::overlay::PaletteOverlay;
use crate::graphics::font::{self, FontKey, Fonts};
use crate::graphics::lighting::light_map::{self, LightMap};
use crate::graphics::{Point, Rect};
use crate::util::SmKey;

pub struct Backend {
    canvas: WindowCanvas,
    palette: Box<Palette>,
    palette_overlay: PaletteOverlay,
    textures: Textures,
}

impl Backend {
    pub fn new(canvas: WindowCanvas, palette: Box<Palette>,
            palette_overlay: PaletteOverlay) -> Self {
        Self {
            canvas,
            palette,
            palette_overlay,
            textures: Textures::new(),
        }
    }

    pub fn new_texture_factory(&self) -> TextureFactory {
        TextureFactory(TextureFactoryInner::Software(self.textures.clone()))
    }

    pub fn into_canvas(self, fonts: Rc<Fonts>) -> Box<dyn Canvas> {
        Box::new(CanvasImpl::new(self, fonts))
    }
}

struct Texture {
    width: i32,
    height: i32,
    data: Box<[u8]>,
}

impl Texture {
    pub fn new(width: i32, height: i32, data: Box<[u8]>) -> Self {
        assert_eq!(data.len(), (width * height) as usize);
        Self {
            data,
            width,
            height,
        }
    }

    pub fn new_empty(width: i32, height: i32, fill_color: u8) -> Self {
        assert!(width > 0 && height > 0);
        let len = (width * height) as usize;
        let data = vec![fill_color; len].into_boxed_slice();

        Self::new(width, height, data)
    }

    pub fn len(&self) -> usize {
        (self.width * self.height) as usize
    }
}

struct TexturesInner {
    handles: SlotMap<SmKey, ()>,
    textures: SecondaryMap<SmKey, Texture>,
    drop_list: Rc<RefCell<Vec<SmKey>>>,
}

impl TexturesInner {
    fn new() -> Self {
        Self {
            handles: SlotMap::with_key(),
            textures: SecondaryMap::new(),
            drop_list: Rc::new(RefCell::new(Vec::new())),
        }
    }

    fn new_texture(&mut self, width: i32, height: i32, data: Box<[u8]>) -> TextureHandle {
        let key = self.handles.insert(());
        self.textures.insert(key, Texture::new(width, height, data));
        TextureHandle(Rc::new(TextureHandleInner {
            key,
            drop_list: self.drop_list.clone(),
        }))
    }

    fn cleanup(&mut self) {
        let mut l = self.drop_list.borrow_mut();
        for key in l.drain(..) {
            self.handles.remove(key);
            self.textures.remove(key);
        }
    }
}

#[derive(Clone)]
pub(in super) struct Textures(Rc<RefCell<TexturesInner>>);

impl Textures {
    fn new() -> Self {
        Textures(Rc::new(RefCell::new(TexturesInner::new())))
    }

    fn cleanup(&self) {
        self.0.borrow_mut().cleanup();
    }
}

impl Textures {
    pub fn new_texture(&self, width: i32, height: i32, data: Box<[u8]>) -> TextureHandle {
        self.0.borrow_mut().new_texture(width, height, data)
    }

    fn get(&self, h: &TextureHandle) -> Ref<Texture> {
        let t = self.0.borrow();
        Ref::map(t, |t| &t.textures[h.0.key])
    }
}

struct CanvasImpl {
    canvas: WindowCanvas,
    palette: Box<Palette>,
    palette_overlay: PaletteOverlay,
    textures: Textures,
    light_map: LightMap,
    back_buf: Texture,
    canvas_texture: SdlTexture,
    clip_rect: Rect,
    fonts: Rc<Fonts>,
}

impl CanvasImpl {
    fn new(backend: Backend, fonts: Rc<Fonts>) -> Self {
        let (w, h) = backend.canvas.window().size();
        let canvas_texture = backend.canvas
            .texture_creator()
            .create_texture_streaming(PixelFormatEnum::RGB24, w, h)
            .unwrap();
        Self {
            canvas: backend.canvas,
            palette: backend.palette,
            palette_overlay: backend.palette_overlay,
            textures: backend.textures,
            light_map: LightMap::new(),
            back_buf: Texture::new_empty(w as i32, h as i32, 0),
            canvas_texture,
            clip_rect: Rect::with_size(0, 0, w as i32, h as i32),
            fonts,
        }
    }

    fn make_translucent(src: u8, dst: u8, trans_color_idx: u8, palette: &Palette,
            grayscale_func: impl Fn(Rgb15) -> u8) -> u8 {
        let alpha = grayscale_func(palette.rgb15(src)) / 4;
        palette.alpha_blend(trans_color_idx, dst, alpha)
    }

    fn do_draw_translucent(&mut self, tex: &TextureHandle, pos: Point, color: Rgb15, light: u32,
            grayscale_func: impl Fn(Rgb15) -> u8) {
        let pal = &self.palette;
        let tex = self.textures.get(tex);
        let light = (light >> 9) as u8;

        let color = pal.color_idx(color);

        Self::do_draw(&mut self.back_buf, pos.x, pos.y, &tex, self.clip_rect,
            |dst, _, _, _, _, src| {
                let color = Self::make_translucent(src, *dst, color, pal,
                    |rgb15| grayscale_func(rgb15));
                *dst = pal.darken(color, light);
            }
        );
    }

    fn compute_draw_rect(dst: &Texture, dst_x: i32, dst_y: i32,
                         src_width: i32, src_height: i32,
                         clip_rect: Rect) -> (Rect, i32, i32) {
        let rect = Rect::with_size(dst_x, dst_y, src_width, src_height)
            .intersect(Rect::with_size(0, 0, dst.width, dst.height))
            .intersect(clip_rect);
        let src_rect = rect.translate(Point::new(-dst_x, -dst_y));
        let dst_x = rect.left;
        let dst_y = rect.top;
        (src_rect, dst_x, dst_y)
    }

    fn do_draw(
            dst: &mut Texture,
            dst_x: i32, dst_y: i32,
            src: &Texture,
            clip_rect: Rect,
            f: impl Fn(&mut u8, i32, i32, i32, i32, u8)) {
        let (src_rect, dst_x, mut dst_y) =
            Self::compute_draw_rect(dst, dst_x, dst_y, src.width, src.height, clip_rect);

        for src_y in src_rect.top..src_rect.bottom {
            let src = &src.data[(src_y * src.width) as usize..];
            let dst = &mut dst.data[(dst_y * dst.width) as usize..];
            let mut dst_x_i = dst_x;
            for src_x in src_rect.left..src_rect.right {
                let src_color_idx = src[src_x as usize];
                if src_color_idx != 0 {
                    let dst_color_idx = &mut dst[dst_x_i as usize];
                    f(dst_color_idx, dst_x_i, dst_y, src_x, src_y, src_color_idx);
                }
                dst_x_i += 1;
            }
            dst_y += 1;
        }
    }
}

impl Canvas for CanvasImpl {
    fn cleanup(&mut self) {
        self.textures.cleanup();
    }

    fn present(&mut self) {
        let pal = &self.palette;
        let pal_overlay = &self.palette_overlay;
        let src = &self.back_buf.data;
        let src_width = self.back_buf.width;
        self.canvas_texture.with_lock(None, |dst, stride| {
            for (src_row, dst_row) in src.chunks(src_width as usize).zip(dst.chunks_mut(stride)) {
                for (&src_pixel, dst_pixel) in src_row.iter().zip(dst_row.chunks_mut(3)) {
                    let rgb = pal_overlay.get(src_pixel)
                        .unwrap_or_else(|| pal.rgb18(src_pixel))
                        .scale::<Color8>();
                    dst_pixel[0] = rgb.r();
                    dst_pixel[1] = rgb.g();
                    dst_pixel[2] = rgb.b();
                }
            }
        }).unwrap();
        self.canvas.copy(&self.canvas_texture, None, None).unwrap();
        self.canvas.present();
    }

    fn update(&mut self, time: Instant) {
        self.palette_overlay.rotate(time);
    }

    fn fonts(&self) -> &Rc<Fonts> {
        &self.fonts
    }

    fn set_clip_rect(&mut self, rect: Rect) {
        self.reset_clip_rect();
        self.clip_rect = rect.intersect(self.clip_rect);
    }

    fn reset_clip_rect(&mut self) {
        let (w, h) = self.canvas.window().size();
        self.clip_rect = Rect::with_size(0, 0, w as i32, h as i32);
    }

    fn clear(&mut self, color: Rgb15) {
        let v = self.palette.color_idx(color);
        for b in self.back_buf.data.iter_mut() {
            *b = v;
        }
    }

    fn draw(&mut self, tex: &TextureHandle, pos: Point, light: u32) {
        let pal = &self.palette;
        let tex = self.textures.get(tex);
        let light = (light >> 9) as u8;

        Self::do_draw(&mut self.back_buf, pos.x, pos.y, &tex, self.clip_rect,
            |dst, _, _, _, _, src| {
                *dst = pal.darken(src, light);
            }
        );
    }

    fn draw_multi_light(&mut self, tex: &TextureHandle, pos: Point, lights: &[u32]) {
        let mut uniform = true;
        for i in 1..light_map::VERTEX_COUNT {
            if lights[i] != lights[i - 1] {
                uniform = false;
                break;
            }
        }
        if uniform {
            self.draw(tex, pos, lights[0]);
            return;
        }

        self.light_map.build(lights);

        let pal = &self.palette;
        let light_map = &self.light_map;

        let tex = self.textures.get(tex);

        Self::do_draw(&mut self.back_buf, pos.x, pos.y, &tex, self.clip_rect,
            |dst, _, _, src_x, src_y, src| {
                let light = light_map.get(src_x, src_y + 2 /* as in original */);
                *dst = pal.darken(src, (light >> 9) as u8);
            }
        );
    }

    fn draw_masked(&mut self, tex: &TextureHandle, pos: Point,
                   mask: &TextureHandle, mask_pos: Point,
                   light: u32) {
        let tex = self.textures.get(tex);
        let mask = self.textures.get(mask);

        let mask_rect = Rect::with_size(mask_pos.x, mask_pos.y, mask.width, mask.height);
        let light = (light >> 9) as u8;

        let pal = &self.palette;

        Self::do_draw(&mut self.back_buf, pos.x, pos.y, &tex, self.clip_rect,
            |dst, dst_x, dst_y, _, _, src| {
                let src = pal.darken(src, light);
                let mask_v = if mask_rect.contains(Point::new(dst_x, dst_y)) {
                    let i = (dst_y - mask_pos.y) * mask.width + dst_x - mask_pos.x;
                    cmp::min(mask.data[i as usize], 128)
                } else {
                    0
                };
                *dst = if mask_v > 0 {
                    let masked_src = pal.darken(src, mask_v);
                    let masked_dst = pal.darken(*dst, 128 - mask_v);
                    pal.blend(masked_src, masked_dst)
                } else {
                    src
                };
            }
        );
    }

    fn draw_masked_color(&mut self, src: Rgb15, dst: Option<Rgb15>, pos: Point,
            mask: &TextureHandle) {
        let mask = self.textures.get(mask);
        let pal = &self.palette;
        let src_color_idx = pal.color_idx(src);
        let dst_color_idx = dst.map(|c| pal.color_idx(c));

        Self::do_draw(&mut self.back_buf, pos.x, pos.y, &mask, self.clip_rect,
            |dst, _, _, _, _, src| {
                let alpha = cmp::min(src, 7);
                *dst = pal.alpha_blend(src_color_idx, dst_color_idx.unwrap_or(*dst), alpha);
            }
        );
    }

    fn draw_highlight(&mut self, color: Rgb15, pos: Point, mask: &TextureHandle) {
        let mask = self.textures.get(mask);
        let pal = &self.palette;
        let color_idx = pal.color_idx(color);

        Self::do_draw(&mut self.back_buf, pos.x, pos.y, &mask, self.clip_rect,
            |dst, _, _, _, _, src| {
                let x = if src != 0 {
                    ((256 - src as u32) >> 4) as u8
                } else {
                    0
                };
                *dst = pal.blend_lookup(color_idx, *dst, x);
            }
        );
    }

    fn draw_translucent(&mut self, tex: &TextureHandle, pos: Point, color: Rgb15, light: u32) {
        self.do_draw_translucent(tex, pos, color, light, Rgb15::grayscale)
    }

    fn draw_translucent_dark(&mut self, tex: &TextureHandle, pos: Point, color: Rgb15, light: u32) {
        self.do_draw_translucent(tex, pos, color, light, Rgb15::grayscale_dark)
    }

    fn draw_outline(&mut self, tex: &TextureHandle, pos: Point, outline: Outline) {
        let src = self.textures.get(tex);
        let (mut src_rect, dst_x, dst_y) =
            Self::compute_draw_rect(&self.back_buf, pos.x - 1, pos.y - 1,
                src.width + 2, src.height + 2,
                self.clip_rect);
        src_rect.right -= 2;
        src_rect.bottom -= 2;
        let dst_width = self.back_buf.width;

        let (color_start, color_end_incl, trans_color_idx) = match outline {
            Outline::Fixed { color, trans_color } => {
                let start = self.palette.color_idx(color);
                let trans_color_idx = trans_color.map(|c| self.palette.color_idx(c));
                (start, start, trans_color_idx)
            },
            Outline::Cycled { start, len } => {
                assert!(start + len > start);
                (start, start + len - 1, None)
            },
        };

        let vert_period = cmp::max(src.height / (color_end_incl as i32 - color_start as i32 + 1), 1);

        // Scan horizontally.
        let mut dst_y_i = dst_y + 1;
        let mut color_idx = color_start;
        for src_y in 0..src_rect.bottom {
            if src_y % vert_period == 0 {
                if color_idx < color_end_incl {
                    color_idx += 1;
                } else {
                    color_idx = color_start;
                }
            }
            if src_y >= src_rect.top {
                let mut outside = true;
                let src = &src.data[(src_y * src.width) as usize..];
                let dst = &mut self.back_buf.data[(dst_y_i * dst_width) as usize..];
                let mut dst_x_i = dst_x;
                for src_x in 0..=src_rect.right {
                    let src_color_idx = if src_x < src_rect.right {
                        src[src_x as usize]
                    } else {
                        0
                    };
                    let dst_x = if src_color_idx != 0 && outside {
                        outside = false;
                        Some(dst_x_i as usize)
                    } else if src_color_idx == 0 && !outside {
                        outside = true;
                        Some(dst_x_i as usize + 1)
                    } else {
                        None
                    };
                    if src_x >= src_rect.left {
                        if let Some(dst_x) = dst_x {
                            dst[dst_x] = if let Some(trans_color_idx) = trans_color_idx {
                                Self::make_translucent(color_idx, dst[dst_x], trans_color_idx,
                                    &self.palette, Rgb15::grayscale)
                            } else {
                                color_idx
                            };
                        }
                    }
                    dst_x_i += 1;
                }
                dst_y_i += 1;
            }
        }

        // Scan vertically.
        let mut dst_x_i = dst_x + 1;
        for src_x in src_rect.left..src_rect.right {
            let mut dst_y_i = dst_y;
            let mut color_idx = color_start;
            let mut outside = true;
            for src_y in 0..=src_rect.bottom {
                if src_y % vert_period == 0 {
                    if color_idx < color_end_incl {
                        color_idx += 1;
                    } else {
                        color_idx = color_start;
                    }
                }
                let src = if src_y < src_rect.bottom {
                    src.data[(src_y * src.width + src_x) as usize]
                } else {
                    0
                };
                let dst_y = if src != 0 && outside {
                    outside = false;
                    Some(dst_y_i)
                } else if src == 0 && !outside {
                    outside = true;
                    Some(dst_y_i + 1)
                } else {
                    None
                };
                if src_y >= src_rect.top {
                    if let Some(dst_y) = dst_y {
                        let dst = &mut self.back_buf.data[(dst_y * dst_width + dst_x_i) as usize];
                        *dst = if let Some(trans_color_idx) = trans_color_idx {
                            Self::make_translucent(color_idx, *dst, trans_color_idx,
                                &self.palette, Rgb15::grayscale)
                        } else {
                            color_idx
                        };
                    }
                    dst_y_i += 1;
                }
            }
            dst_x_i += 1;
        }
    }

    fn draw_text(&mut self, text: &bstr, pos: Point, font: FontKey, color: Rgb15,
            options: &font::DrawOptions) {
        let fonts = self.fonts.clone();
        fonts.get(font).draw(self, text, pos, color, options);
    }
}
